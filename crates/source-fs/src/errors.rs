use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur when loading or using the FileSystem.
#[derive(Error, Debug)]
pub enum FileSystemError {
    /// The `gameinfo.txt` file was not found in the specified path.
    #[error("gameinfo.txt not found in {0}")]
    GameInfoNotFound(PathBuf),

    /// The provided game path is invalid (e.g., it has no parent or file name).
    #[error("invalid game path (missing parent or file name): {0}")]
    InvalidGamePath(PathBuf),

    /// The `gameinfo.txt` file could not be parsed.
    #[error("failed to parse gameinfo.txt")]
    GameInfoParseError,

    #[error("failed to read `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to mount pack file `{path}`: {source}")]
    Pack {
        path: PathBuf,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("failed to read pack entry `{entry}`: {source}")]
    PackEntry {
        entry: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("`{path}` is not valid UTF-8: {source}")]
    Utf8 {
        path: String,
        #[source]
        source: std::string::FromUtf8Error,
    },

    /// An error occurred while using `steamlocate`.
    #[cfg(feature = "steam")]
    #[error("steamlocate error: {0}")]
    SteamLocateError(#[from] steamlocate::Error),

    /// The specified Steam App ID was not found.
    #[cfg(feature = "steam")]
    #[error("steam app {0} not found")]
    SteamAppNotFound(u32),

    /// The Steam installation could not be found.
    #[cfg(feature = "steam")]
    #[error("steam installation not found")]
    SteamNotFound,
}

impl FileSystemError {
    pub(crate) fn pack(
        path: PathBuf,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Pack {
            path,
            source: Box::new(source),
        }
    }

    pub(crate) fn pack_entry(
        entry: String,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::PackEntry {
            entry,
            source: Box::new(source),
        }
    }
}
