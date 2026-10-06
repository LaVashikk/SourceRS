use std::error::Error;
use std::io::{Read, Seek};
use std::path::Path;

#[cfg(feature = "vpk")]
pub use source_vpk::Vpk;

/// Archive backend mounted by [`crate::FileSystem`]
pub trait PackFile {
    type Error: Error + Send + Sync + 'static;
    type Reader<'a>: Read + Seek + 'a
    where
        Self: 'a;

    /// Opens an archive, or returns `Ok(None)` when this backend intentionally
    /// ignores pack files (as the loose-file-only backend does)
    fn open<P: AsRef<Path>>(path: P) -> std::result::Result<Option<Self>, Self::Error>
    where
        Self: Sized;

    fn has_entry(&self, path: &str) -> bool;

    /// Opens one entry without forcing the complete payload into memory
    fn open_entry<'a>(&'a self, path: &str) -> Result<Self::Reader<'a>, Self::Error>;
}

/// A dummy implementation that ignores VPK files.
/// Forces the FileSystem to read exclusively from physical disk directories
#[derive(Debug, Clone, Copy)]
pub struct DummyVpk;

impl PackFile for DummyVpk {
    type Error = std::convert::Infallible;
    type Reader<'a> = std::io::Cursor<Vec<u8>>;

    fn open<P: AsRef<Path>>(_path: P) -> std::result::Result<Option<Self>, Self::Error> {
        Ok(None)
    }

    fn has_entry(&self, _path: &str) -> bool {
        false
    }

    fn open_entry<'a>(&'a self, _path: &str) -> Result<Self::Reader<'a>, Self::Error> {
        Ok(std::io::Cursor::new(Vec::new()))
    }
}

#[cfg(feature = "vpk")]
impl PackFile for source_vpk::Vpk {
    type Error = source_vpk::Error;
    type Reader<'a> = source_vpk::EntryReader;

    fn open<P: AsRef<Path>>(path: P) -> std::result::Result<Option<Self>, Self::Error> {
        source_vpk::Vpk::open(path).map(Some)
    }

    fn has_entry(&self, path: &str) -> bool {
        self.contains(path)
    }

    fn open_entry<'a>(&'a self, path: &str) -> Result<Self::Reader<'a>, Self::Error> {
        source_vpk::Vpk::open_entry(self, path)
    }
}

/// Pack backend picked by features: [`Vpk`] with `vpk`, otherwise [`DummyVpk`] (loose files only)
#[cfg(feature = "vpk")]
pub type DefaultPack = Vpk;
/// Pack backend picked by features: [`Vpk`] with `vpk`, otherwise [`DummyVpk`] (loose files only)
#[cfg(not(feature = "vpk"))]
pub type DefaultPack = DummyVpk;
