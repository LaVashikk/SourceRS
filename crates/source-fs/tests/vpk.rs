use std::{fs, io::Read};

use source_fs::{FileLocation, Error, Mount, create_fs};
use tempfile::tempdir;

const SIGNATURE: u32 = 0x55aa_1234;
const DIRECTORY_ARCHIVE_INDEX: u16 = 0x7fff;
const ENTRY_TERMINATOR: u16 = 0xffff;

#[test]
fn reads_assets_from_gameinfo_vpk_search_paths() {
    let root = tempdir().unwrap();
    let game_dir = root.path().join("game");
    fs::create_dir(&game_dir).unwrap();
    fs::write(
        game_dir.join("gameinfo.txt"),
        r#"
        GameInfo
        {
            FileSystem
            {
                SearchPaths
                {
                    Game game/pak01_dir.vpk
                }
            }
        }
        "#,
    )
    .unwrap();

    let payload = b"VertexLitGeneric { $basetexture example/brick }";
    fs::write(
        game_dir.join("pak01_dir.vpk"),
        one_file_vpk("materials/example/brick.vmt", payload),
    )
    .unwrap();

    let file_system = create_fs(&game_dir).unwrap();
    let mut reader = file_system
        .open("materials/example/brick.vmt", "game")
        .unwrap();
    let mut streamed = Vec::new();
    reader.read_to_end(&mut streamed).unwrap();
    assert_eq!(streamed, payload);

    assert_eq!(
        file_system
            .read("MATERIALS\\EXAMPLE\\BRICK.VMT", "game")
            .unwrap(),
        payload
    );
    assert_eq!(
        file_system
            .read("materials/example/brick.vmt", "game")
            .unwrap(),
        payload
    );
}

#[test]
fn reports_invalid_vpks_during_mount() {
    let root = tempdir().unwrap();
    let game_dir = root.path().join("game");
    fs::create_dir(&game_dir).unwrap();
    fs::write(
        game_dir.join("gameinfo.txt"),
        "SearchPaths\n{\n    Game game/broken_dir.vpk\n}\n",
    )
    .unwrap();
    fs::write(game_dir.join("broken_dir.vpk"), b"not a vpk").unwrap();

    assert!(matches!(
        create_fs(&game_dir).unwrap_err(),
        Error::Pack { .. }
    ));
}

fn one_file_vpk(path: &str, payload: &[u8]) -> Vec<u8> {
    let (directory, name) = path.rsplit_once('/').unwrap_or((" ", path));
    let (stem, extension) = name.rsplit_once('.').unwrap_or((name, " "));
    let mut tree = Vec::new();

    push_c_string(&mut tree, extension);
    push_c_string(&mut tree, directory);
    push_c_string(&mut tree, stem);
    tree.extend_from_slice(&crc32fast::hash(payload).to_le_bytes());
    tree.extend_from_slice(&0_u16.to_le_bytes());
    tree.extend_from_slice(&DIRECTORY_ARCHIVE_INDEX.to_le_bytes());
    tree.extend_from_slice(&0_u32.to_le_bytes());
    tree.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    tree.extend_from_slice(&ENTRY_TERMINATOR.to_le_bytes());
    tree.extend_from_slice(&[0, 0, 0]);

    let mut output = Vec::new();
    output.extend_from_slice(&SIGNATURE.to_le_bytes());
    output.extend_from_slice(&1_u32.to_le_bytes());
    output.extend_from_slice(&(tree.len() as u32).to_le_bytes());
    output.extend_from_slice(&tree);
    output.extend_from_slice(payload);
    output
}

fn push_c_string(output: &mut Vec<u8>, value: &str) {
    output.extend_from_slice(value.as_bytes());
    output.push(0);
}

fn write_gameinfo(game_dir: &std::path::Path, search_paths: &str) {
    fs::create_dir_all(game_dir).unwrap();
    fs::write(
        game_dir.join("gameinfo.txt"),
        format!("GameInfo {{ FileSystem {{ SearchPaths {{\n{search_paths}\n}} }} }}"),
    )
    .unwrap();
}

fn write_file(path: &std::path::Path, data: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, data).unwrap();
}

#[test]
fn search_order_follows_gameinfo_lines() {
    let root = tempdir().unwrap();
    let base = root.path();
    write_gameinfo(
        &base.join("portal"),
        "game+mod portal/custom/*\n\
         game+mod portal/portal_pak.vpk\n\
         game+game_write portal\n\
         game hl2",
    );
    write_file(&base.join("portal/custom/a_loose/shared.txt"), b"custom");
    write_file(&base.join("portal/custom/b_mod.vpk"), &one_file_vpk("only_in_custom_vpk.txt", b"custom vpk"));
    // A data chunk with no `_dir.vpk`: mounting it would fail the whole load
    write_file(&base.join("portal/custom/orphan_000.vpk"), b"not a vpk");
    write_file(&base.join("portal/portal_pak_dir.vpk"), &one_file_vpk("shared.txt", b"pak"));
    write_file(&base.join("hl2/shared.txt"), b"hl2 loose");

    let fs = create_fs(base.join("portal")).unwrap();

    assert_eq!(fs.read("shared.txt", "game").unwrap(), b"custom");
    assert!(matches!(
        fs.find("only_in_custom_vpk.txt", "game"),
        Some(FileLocation::Packed { .. })
    ));

    // portal_pak (line 2) sits above the hl2 dir (line 4)
    fs::remove_dir_all(base.join("portal/custom/a_loose")).unwrap();
    assert_eq!(fs.read("shared.txt", "game").unwrap(), b"pak");

    // portal_pak_dir.vpk is reachable both explicitly and through `portal`, mounted once
    let packs = fs.mounts("game").iter().filter(|m| matches!(m, Mount::Pack(_))).count();
    assert_eq!(packs, 2);
}

#[test]
fn directory_packs_sit_above_its_loose_files() {
    let root = tempdir().unwrap();
    let game_dir = root.path().join("game");
    write_gameinfo(&game_dir, "game |gameinfo_path|.");
    write_file(&game_dir.join("pak01_dir.vpk"), &one_file_vpk("scripts/a.txt", b"packed"));
    write_file(&game_dir.join("scripts/a.txt"), b"loose");

    let fs = create_fs(&game_dir).unwrap();
    let location = fs.find("scripts/a.txt", "game").unwrap();
    assert!(matches!(location, FileLocation::Packed { ref entry, .. } if entry == "scripts/a.txt"));
    assert_eq!(location.read().unwrap(), b"packed");
}
