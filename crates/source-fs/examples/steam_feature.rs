
#[cfg(feature = "steam")]
fn main() {
    use source_fs::{DummyVpk, FileSystem, FileSystemOptions, SimpleGameInfo};

    let options = FileSystemOptions::default();
    let fs = FileSystem::<DummyVpk>::load_from_app_id::<SimpleGameInfo>(620, "portal2", &options).unwrap();

    let file = fs.read_str("scripts/vscripts/mapspawn.nut", "game", false).unwrap();
    println!("Found file: \n{}", file);
}

#[cfg(not(feature = "steam"))]
fn main() {
    panic!("'Steam' feature is not enabled");
}
