pub mod fs;
pub mod providers;
pub mod traits;
pub(crate) mod utils;

pub use fs::{FileSystem, FileSystemOptions};
pub use providers::{DummyVpk, P2GameInfo, SimpleGameInfo};
pub use traits::{GameInfoProvider, PackFile};


use std::path::Path;

/// Creates a FileSystem using the SimpleGameInfo provider,
/// without loading any VPK files.
pub fn create_fs<P: AsRef<Path>>(game_dir: P) -> FileSystem<DummyVpk> {
    create_fs_custom::<SimpleGameInfo, P>(game_dir)
}

/// Creates a FileSystem using a custom GameInfoProvider,
/// without loading any VPK files.
pub fn create_fs_custom<G: GameInfoProvider, P: AsRef<Path>>(game_dir: P) -> FileSystem<DummyVpk> {
    let options = FileSystemOptions::default();
    FileSystem::<DummyVpk>::load_from_path::<G>(game_dir.as_ref(), &options).unwrap()
}
