use std::error::Error;
use std::io::{Read, Seek};
use std::path::Path;

/// Archive backend mounted by [`crate::FileSystem`].
pub trait PackFile {
    type Error: Error + Send + Sync + 'static;
    type Reader<'a>: Read + Seek + 'a
    where
        Self: 'a;

    /// Opens an archive, or returns `Ok(None)` when this backend intentionally
    /// ignores pack files (as the loose-file-only backend does).
    fn open<P: AsRef<Path>>(path: P) -> Result<Option<Self>, Self::Error>
    where
        Self: Sized;

    fn has_entry(&self, path: &str) -> bool;

    /// Opens one entry without forcing the complete payload into memory.
    fn open_entry<'a>(&'a self, path: &str) -> Result<Self::Reader<'a>, Self::Error>;
}

/// Provides parsing capabilities for `gameinfo.txt` to extract search paths.
pub trait GameInfoProvider {
    /// Parses the given `gameinfo.txt` file and returns a list of search paths.
    /// The return value is a list of tuples: `(search_path_id, physical_path)`.
    fn get_search_paths<P: AsRef<Path>>(path: P) -> Option<Vec<(String, String)>>;
}
