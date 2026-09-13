use std::{fs, io::Read};

use source_fs::{FileSystemError, create_vpk_fs};
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

    let file_system = create_vpk_fs(&game_dir).unwrap();
    let mut reader = file_system
        .open("materials/example/brick.vmt", "game", false)
        .unwrap()
        .unwrap();
    let mut streamed = Vec::new();
    reader.read_to_end(&mut streamed).unwrap();
    assert_eq!(streamed, payload);

    assert_eq!(
        file_system
            .read("MATERIALS\\EXAMPLE\\BRICK.VMT", "game", false)
            .unwrap(),
        Some(payload.to_vec())
    );
    assert_eq!(
        file_system
            .read("materials/example/brick.vmt", "game", true)
            .unwrap(),
        Some(payload.to_vec())
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
        create_vpk_fs(&game_dir).unwrap_err(),
        FileSystemError::Pack { .. }
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
