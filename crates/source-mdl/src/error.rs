use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FileKind {
    Mdl,
    Vvd,
    Vtx,
}

impl std::fmt::Display for FileKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Mdl => "MDL",
            Self::Vvd => "VVD",
            Self::Vtx => "VTX",
        })
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error for `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("invalid {file} signature 0x{actual:08x}; expected 0x{expected:08x}")]
    InvalidSignature {
        file: FileKind,
        expected: u32,
        actual: u32,
    },

    #[error("unsupported {file} version {version}")]
    UnsupportedVersion { file: FileKind, version: i32 },

    #[error("{file} uses {feature}, which this parser does not read")]
    UnsupportedFeature { file: FileKind, feature: &'static str },

    /// A byte range of the file, or an element range such as a strip's slice of its group
    #[error("{file} {context} range {offset}..{end} is outside 0..{size}")]
    OutOfBounds {
        file: FileKind,
        context: &'static str,
        offset: usize,
        end: usize,
        size: usize,
    },

    #[error("invalid {field} value {value} in {file}: {reason}")]
    InvalidField {
        file: FileKind,
        field: &'static str,
        value: i64,
        reason: &'static str,
    },

    #[error("invalid {context} index {index}; valid count is {count}")]
    InvalidIndex {
        context: &'static str,
        index: i64,
        count: usize,
    },

    #[error("{file} checksum 0x{actual:08x} does not match MDL checksum 0x{expected:08x}")]
    ChecksumMismatch {
        file: FileKind,
        expected: u32,
        actual: u32,
    },

    #[error("{context} mismatch: expected {expected}, found {actual}")]
    StructureMismatch {
        context: &'static str,
        expected: usize,
        actual: usize,
    },

    #[cfg(feature = "model_system")]
    #[error(transparent)]
    FileSystem(#[from] source_fs::Error),

    #[cfg(feature = "model_system")]
    #[error("failed to resolve material `{path}`: {source}")]
    Material {
        path: String,
        #[source]
        source: source_vmt::Error,
    },
}

impl Error {
    pub(crate) fn io(path: impl AsRef<Path>, source: io::Error) -> Self {
        Self::Io { path: path.as_ref().to_path_buf(), source }
    }

    pub(crate) fn index(context: &'static str, index: usize, count: usize) -> Self {
        Self::InvalidIndex { context, index: index as i64, count }
    }
}
