use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when loading or using the FileSystem.
#[derive(Error, Debug)]
pub enum Error {
    #[error("file `{path}` not found in search path `{search_path}`")]
    NotFound {
        path: String,
        search_path: String,
    },

    #[error("gameinfo.txt not found in {0}")]
    GameInfoNotFound(PathBuf),

    #[error("invalid game path (missing parent or file name): {0}")]
    InvalidGamePath(PathBuf),

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

    #[cfg(feature = "steam")]
    #[error("steamlocate error: {0}")]
    SteamLocateError(#[from] steamlocate::Error),

    #[cfg(feature = "steam")]
    #[error("steam app {0} not found")]
    SteamAppNotFound(u32),

    #[cfg(feature = "steam")]
    #[error("steam installation not found")]
    SteamNotFound,
}

impl Error {
    pub(crate) fn not_found(
        path: &str,
        search_path: &str,
    ) -> Self {
        Self::NotFound {
            path: path.to_string(),
            search_path: search_path.to_string(),
        }
    }

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
