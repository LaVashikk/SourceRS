use source_fs::{FileSystem, FileSystemOptions, providers::{DummyVpk, P2GameInfo, SimpleGameInfo}};
use std::path::Path;

fn create_fs<P: AsRef<Path>>(game_dir: P) -> Option<FileSystem<DummyVpk>> {
    let options = FileSystemOptions::default();
    FileSystem::<DummyVpk>::load_from_path::<SimpleGameInfo>(game_dir.as_ref(), &options)
}

#[test]
fn simple_game_test() {
    let fs = create_fs("tests/games/simple/game").expect("Failed to create FileSystem");
    assert!(!fs.search_path_dirs().is_empty());

    let data = fs.read("scripts/test.txt", "game", false);
    assert!(data.is_some());
    assert_eq!(String::from_utf8_lossy(&data.unwrap()), "Hello world");

    let data = fs.read("other_test.txt", "game", false);
    assert!(data.is_some());
    assert_eq!(String::from_utf8_lossy(&data.unwrap()), "File in other_folder");
}

#[test]
fn simple_2_game_test() {
    let fs = create_fs("tests/games/simple_inside").expect("Failed to create FileSystem");
    assert!(!fs.search_path_dirs().is_empty());

    let data = fs.read("scripts/test.txt", "game", false);
    assert!(data.is_some());
    assert_eq!(String::from_utf8_lossy(&data.unwrap()), "Hello world\n");

    let data = fs.read("other_test.txt", "game", false);
    assert!(data.is_some());
    assert_eq!(String::from_utf8_lossy(&data.unwrap()), "File in other_folder\n");
}

#[test]
fn portal_game_test() {
    let fs = create_fs("tests/games/portal/portal").expect("Failed to create FileSystem");
    assert!(!fs.search_path_dirs().is_empty());

    let data = fs.read("mod_test.txt", "game", false);
    assert!(data.is_some());
    assert_eq!(String::from_utf8_lossy(&data.unwrap()), "Mod file in portal/custom/test_mod\n");

    let data = fs.read("nothing.txt", "game", false);
    assert!(data.is_some());
    assert_eq!(String::from_utf8_lossy(&data.unwrap()), "nothing\n");
}

// #[test]
// fn hl2_game_test() {
//     let fs = create_fs("tests/games/portal/portal").expect("Failed to create FileSystem");
//     assert!(!fs.search_path_dirs().is_empty());

//     let data = fs.read("mod_test.txt", "game", false);
//     assert!(data.is_some());
//     assert_eq!(String::from_utf8_lossy(&data.unwrap()), "Mod file in portal/custom/test_mod\n");

//     let data = fs.read("nothing.txt", "game", false);
//     assert!(data.is_some());
//     assert_eq!(String::from_utf8_lossy(&data.unwrap()), "nothing\n");
// }

#[test]
fn portal2_game_test() {
    // Portal 2 has a unique feature: DLC folders.
    // They aren't added to SearchPaths;
    // the game automatically mounts the content if it exists, incrementing the DLC number

    let path = Path::new("tests/games/portal2/portal2");
    let fs = create_fs(&path).expect("Failed to create FileSystem");
    assert!(!fs.search_path_dirs().is_empty());

    let data = fs.read("file1.txt", "game", false);
    assert!(data.is_none());

    let options = FileSystemOptions::default();
    let fs_p2 = FileSystem::<DummyVpk>::load_from_path::<P2GameInfo>(path, &options).expect("Failed to create P2 FileSystem");

    let data = fs_p2.read("file1.txt", "game", false);
    assert!(data.is_some());
    assert_eq!(String::from_utf8_lossy(&data.unwrap()), "1\n");

    let data = fs_p2.read("file2.txt", "game", false);
    assert!(data.is_some());
    assert_eq!(String::from_utf8_lossy(&data.unwrap()), "2\n");

    let data = fs_p2.read("file3.txt", "game", false);
    assert!(data.is_some());
    assert_eq!(String::from_utf8_lossy(&data.unwrap()), "3\n");

}
