pub mod errors;
pub mod fs;
pub mod providers;
pub mod traits;
pub(crate) mod utils;

pub use errors::FileSystemError;
pub use fs::{FileReader, FileSystem, FileSystemOptions};
pub use providers::{DummyVpk, P2GameInfo, SimpleGameInfo};
#[cfg(feature = "vpk")]
pub use source_vpk::Vpk;
pub use traits::{GameInfoProvider, PackFile};
use std::path::Path;

/// Default pack backend: [`Vpk`] with feature `vpk`, otherwise [`DummyVpk`]
#[cfg(feature = "vpk")]
pub type DefaultPack = Vpk;
#[cfg(not(feature = "vpk"))]
pub type DefaultPack = DummyVpk;

#[cfg(feature = "vpk")]
pub type VpkFileSystem = FileSystem<Vpk>;

/// Creates a FileSystem using the SimpleGameInfo provider,
/// without loading any VPK files.
pub fn create_fs<P: AsRef<Path>>(game_dir: P) -> Result<FileSystem<DummyVpk>, FileSystemError> {
    create_fs_custom::<SimpleGameInfo, P>(game_dir)
}

/// Creates a FileSystem using a custom GameInfoProvider,
/// without loading any VPK files.
pub fn create_fs_custom<G: GameInfoProvider, P: AsRef<Path>>(game_dir: P) -> Result<FileSystem<DummyVpk>, FileSystemError> {
    let options = FileSystemOptions::default();
    FileSystem::<DummyVpk>::load_from_path::<G>(game_dir.as_ref(), &options)
}

/// Creates a filesystem that mounts VPK search paths from `gameinfo.txt`.
#[cfg(feature = "vpk")]
pub fn create_vpk_fs<P: AsRef<Path>>(game_dir: P) -> Result<VpkFileSystem, FileSystemError> {
    create_vpk_fs_custom::<SimpleGameInfo, P>(game_dir)
}

/// Creates a VPK-enabled filesystem with a custom gameinfo provider.
#[cfg(feature = "vpk")]
pub fn create_vpk_fs_custom<G: GameInfoProvider, P: AsRef<Path>>(
    game_dir: P,
) -> Result<VpkFileSystem, FileSystemError> {
    let options = FileSystemOptions::default();
    FileSystem::<Vpk>::load_from_path::<G>(game_dir.as_ref(), &options)
}
