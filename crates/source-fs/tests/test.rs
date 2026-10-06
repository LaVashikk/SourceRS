use source_fs::{DummyVpk, Error, FileSystem, FileSystemOptions, providers::{P2GameInfo, SimpleGameInfo, SimpleWithMount}};
use std::path::Path;

fn create_fs<P: AsRef<Path>>(game_dir: P) -> Result<FileSystem<DummyVpk>, Error> {
    let options = FileSystemOptions::default();
    FileSystem::<DummyVpk>::load_from_path::<SimpleGameInfo>(game_dir.as_ref(), &options)
}

#[test]
fn simple_game_test() {
    let fs = create_fs("tests/games/simple/game").expect("Failed to create FileSystem");
    assert!(!fs.search_paths().is_empty());

    let data = fs.read("scripts/test.txt", "game").unwrap();
    assert_eq!(String::from_utf8_lossy(&data), "Hello world");

    let data = fs.read("other_test.txt", "game").unwrap();
    assert_eq!(String::from_utf8_lossy(&data), "File in other_folder");
}

#[test]
fn simple_2_game_test() {
    let fs = create_fs("tests/games/simple_inside").expect("Failed to create FileSystem");
    assert!(!fs.search_paths().is_empty());

    let data = fs.read("scripts/test.txt", "game").unwrap();
    assert_eq!(String::from_utf8_lossy(&data), "Hello world\n");

    let data = fs.read("other_test.txt", "game").unwrap();
    assert_eq!(String::from_utf8_lossy(&data), "File in other_folder\n");
}

#[test]
fn portal_game_test() {
    let fs = create_fs("tests/games/portal/portal").expect("Failed to create FileSystem");
    assert!(!fs.search_paths().is_empty());

    let data = fs.read("mod_test.txt", "game").unwrap();
    assert_eq!(String::from_utf8_lossy(&data), "Mod file in portal/custom/test_mod\n");

    let data = fs.read("nothing.txt", "game").unwrap();
    assert_eq!(String::from_utf8_lossy(&data), "nothing\n");

    let data = fs.read("something_hl2", "game").unwrap();
    assert_eq!(String::from_utf8_lossy(&data), "123\n");
}

#[test]
fn mount_game_test() {
    let path = Path::new("tests/games/gmod/garrysmod");
    let fs = FileSystem::<DummyVpk>::load_from_path::<SimpleWithMount>(path, &FileSystemOptions::default()).expect("Failed to create FileSystem");

    // this should be gmod/garrysmod/
    let data = fs.read("cfg/mount.cfg", "game").unwrap();
    assert!(!data.is_empty());

    // this should be HL2 content in `portal/hl2/` (from mount.cfg)
    let data = fs.read("something_hl2", "game").unwrap();
    assert_eq!(String::from_utf8_lossy(&data), "123\n");

}

#[test]
fn portal2_game_test() {
    // Portal 2 has a unique feature: DLC folders.
    // They aren't added to SearchPaths;
    // the game automatically mounts the content if it exists, incrementing the DLC number

    let path = Path::new("tests/games/portal2/portal2");
    let fs = create_fs(path).expect("Failed to create FileSystem");
    assert!(!fs.search_paths().is_empty());

    assert!(matches!(fs.read("file1.txt", "game"), Err(Error::NotFound { .. })));
    assert!(!fs.exists("file1.txt", "game"));

    let options = FileSystemOptions::default();
    let fs_p2 = FileSystem::<DummyVpk>::load_from_path::<P2GameInfo>(path, &options).expect("Failed to create P2 FileSystem");

    let data = fs_p2.read("file1.txt", "game").unwrap();
    assert_eq!(String::from_utf8_lossy(&data), "1\n");

    let data = fs_p2.read("file2.txt", "game").unwrap();
    assert_eq!(String::from_utf8_lossy(&data), "2\n");

    let data = fs_p2.read("file3.txt", "game").unwrap();
    assert_eq!(String::from_utf8_lossy(&data), "3\n");

}

fn dirs(fs: &FileSystem<DummyVpk>, id: &str) -> Vec<std::path::PathBuf> {
    fs.mounts(id)
        .iter()
        .map(|mount| match mount {
            source_fs::Mount::Dir(dir) => dir.clone(),
            source_fs::Mount::Pack(_) => panic!("DummyVpk mounts no packs"),
        })
        .collect()
}

#[test]
fn portal2_matches_engine_path_output() {
    let root = tempfile::tempdir().unwrap();
    let base = root.path().canonicalize().unwrap();
    for dir in ["portal2", "portal2_dlc1", "portal2_dlc2", "portal2_dlc4", "update", "platform"] {
        std::fs::create_dir(base.join(dir)).unwrap();
    }
    std::fs::write(
        base.join("portal2/gameinfo.txt"),
        "GameInfo { FileSystem { SearchPaths { Game |gameinfo_path|. } } }",
    )
    .unwrap();

    let fs = FileSystem::<DummyVpk>::load_from_path::<P2GameInfo>(&base.join("portal2"), &FileSystemOptions::default())
        .unwrap();

    // dlc4 is past the gap after dlc2, the exe never reaches it
    let expected: Vec<_> = ["update", "portal2_dlc2", "portal2_dlc1", "portal2"].map(|d| base.join(d)).into();
    assert_eq!(dirs(&fs, "mod"), expected);
    let mut expected_game = expected;
    expected_game.push(base.join("platform"));
    assert_eq!(dirs(&fs, "game"), expected_game);
}

#[test]
fn sourcemod_resolves_against_base_dir() {
    let root = tempfile::tempdir().unwrap();
    let base = root.path();
    std::fs::create_dir_all(base.join("sourcemods/mymod")).unwrap();
    std::fs::create_dir_all(base.join("sdk_base/hl2")).unwrap();
    std::fs::write(base.join("sdk_base/hl2/base.txt"), "from base").unwrap();
    std::fs::write(
        base.join("sourcemods/mymod/gameinfo.txt"),
        "GameInfo { FileSystem { SearchPaths {\n game |gameinfo_path|.\n game |all_source_engine_paths|hl2\n} } }",
    )
    .unwrap();

    let options = FileSystemOptions {
        base_dir: Some(base.join("sdk_base")),
        ..Default::default()
    };
    let fs = FileSystem::<DummyVpk>::load_from_path::<SimpleGameInfo>(&base.join("sourcemods/mymod"), &options).unwrap();
    assert_eq!(fs.read_str("base.txt", "game").unwrap(), "from base");
}

#[test]
fn asset_path_hardcore_test() {
    use std::borrow::Cow;
    use source_fs::asset_path;

    // 1. Happy path: zero allocations (Cow::Borrowed)
    let p = asset_path("materials/concrete/wall.vmt", "materials", "vmt");
    assert!(matches!(p, Cow::Borrowed(_)));
    assert_eq!(p, "materials/concrete/wall.vmt");

    let p = asset_path("/materials/concrete/wall.vmt", "materials", "vmt");
    assert!(matches!(p, Cow::Borrowed(_)));
    assert_eq!(p, "materials/concrete/wall.vmt");

    let p = asset_path("models/props/crate.mdl", "models", "mdl");
    assert!(matches!(p, Cow::Borrowed(_)));
    assert_eq!(p, "models/props/crate.mdl");

    // 2. Missing prefix (folder)
    let p = asset_path("concrete/wall.vmt", "materials", "vmt");
    assert!(matches!(p, Cow::Owned(_)));
    assert_eq!(p, "materials/concrete/wall.vmt");

    // 3. Folder with slash or backslash
    let p = asset_path("concrete/wall.vmt", "materials/", "vmt");
    assert_eq!(p, "materials/concrete/wall.vmt");

    let p = asset_path("concrete/wall.vmt", "materials\\", "vmt");
    assert_eq!(p, "materials/concrete/wall.vmt");

    // 4. Missing suffix (extension)
    let p = asset_path("materials/concrete/wall", "materials", "vmt");
    assert!(matches!(p, Cow::Owned(_)));
    assert_eq!(p, "materials/concrete/wall.vmt");

    // 5. Extension with leading dot
    let p = asset_path("materials/concrete/wall", "materials", ".vmt");
    assert_eq!(p, "materials/concrete/wall.vmt");

    // 6. Both missing
    let p = asset_path("concrete/wall", "materials", "vmt");
    assert_eq!(p, "materials/concrete/wall.vmt");

    let p = asset_path("/concrete/wall", "materials/", ".vmt");
    assert_eq!(p, "materials/concrete/wall.vmt");

    // 7. Case insensitivity (preventing duplicate prefixes or suffixes)
    let p = asset_path("Materials/concrete/wall.vmt", "materials", "vmt");
    assert!(matches!(p, Cow::Borrowed(_)));
    assert_eq!(p, "Materials/concrete/wall.vmt");

    let p = asset_path("materials/concrete/wall.VMT", "materials", "vmt");
    assert!(matches!(p, Cow::Borrowed(_)));
    assert_eq!(p, "materials/concrete/wall.VMT");

    let p = asset_path("MATERIALS/CONCRETE/WALL.VMT", "materials", "vmt");
    assert!(matches!(p, Cow::Borrowed(_)));
    assert_eq!(p, "MATERIALS/CONCRETE/WALL.VMT");

    // 8. Lookalike prefix/suffix (no false positives)
    let p = asset_path("materials_hd/wall.vmt", "materials", "vmt");
    assert_eq!(p, "materials/materials_hd/wall.vmt");

    let p = asset_path("materials/wallvmt", "materials", "vmt");
    assert_eq!(p, "materials/wallvmt.vmt");

    // 9. Empty folder or empty extension
    let p = asset_path("scripts/game_sounds", "", "txt");
    assert_eq!(p, "scripts/game_sounds.txt");

    let p = asset_path("materials/custom_file", "materials", "");
    assert!(matches!(p, Cow::Borrowed(_)));
    assert_eq!(p, "materials/custom_file");
}

#[test]
fn not_found_error_context_test() {
    let fs = create_fs("tests/games/simple/game").unwrap();
    let err = fs.read("non_existent_file.txt", "game").unwrap_err();

    match err {
        Error::NotFound { path, search_path } => {
            assert_eq!(path, "non_existent_file.txt");
            assert_eq!(search_path, "game");
        }
        other => panic!("expected Error::NotFound, got: {other:?}"),
    }

    let err_str = fs.read("ghost.vmt", "custom_path").unwrap_err().to_string();
    assert!(err_str.contains("ghost.vmt"));
    assert!(err_str.contains("custom_path"));
}
