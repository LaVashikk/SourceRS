use std::path::Path;

/// Represents a loaded VPK or generic pack file archive.
pub trait PackFile {
    /// Opens a pack file from the given physical path.
    fn open<P: AsRef<Path>>(path: P) -> Option<Self>
    where
        Self: Sized;

    /// Checks if a given file entry exists within the pack file.
    fn has_entry(&self, path: &str) -> bool;

    /// Reads the specified entry's binary data from the pack file.
    fn read_entry(&self, path: &str) -> Option<Vec<u8>>;
}

/// Provides parsing capabilities for `gameinfo.txt` to extract search paths.
pub trait GameInfoProvider {
    /// Parses the given `gameinfo.txt` file and returns a list of search paths.
    /// The return value is a list of tuples: `(search_path_id, physical_path)`.
    fn get_search_paths<P: AsRef<Path>>(path: P) -> Option<Vec<(String, String)>>;
}
