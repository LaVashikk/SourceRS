use std::path::Path;

mod errors;
mod fs;
mod gameinfo;
mod load;
mod mount;
mod pack;
pub mod providers;
pub(crate) mod utils;

pub use errors::{Error, Result};
pub use fs::{FileSystem, asset_path};
pub use load::FileSystemOptions;
pub use mount::{FileLocation, FileReader, Mount};
pub use pack::{DefaultPack, DummyVpk, PackFile};
#[cfg(feature = "vpk")]
pub use source_vpk::Vpk;

/// Loads a filesystem from a game dir with the [`SimpleGameInfo`](providers::SimpleGameInfo) provider.
pub fn create_fs<P: AsRef<Path>>(game_dir: P) -> Result<FileSystem> {
    create_fs_custom::<providers::SimpleGameInfo, P>(game_dir)
}

/// Loads a filesystem from a game dir with a custom [`GameInfoProvider`](providers::GameInfoProvider).
pub fn create_fs_custom<G: providers::GameInfoProvider, P: AsRef<Path>>(game_dir: P) -> Result<FileSystem> {
    FileSystem::load_from_path::<G>(game_dir.as_ref(), &FileSystemOptions::default())
}
