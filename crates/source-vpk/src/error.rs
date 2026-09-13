use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Section {
    Header,
    Tree,
    FileData,
    ArchiveMd5,
    OtherMd5,
    Signature,
}

impl std::fmt::Display for Section {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Header => "header",
            Self::Tree => "directory tree",
            Self::FileData => "file data",
            Self::ArchiveMd5 => "archive MD5 section",
            Self::OtherMd5 => "other MD5 section",
            Self::Signature => "signature section",
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

    #[error("invalid VPK signature 0x{0:08x}")]
    InvalidSignature(u32),

    #[error("unsupported VPK version {0}")]
    UnsupportedVersion(u32),

    #[error(
        "unexpected end of {section} at byte {offset}: need {needed} bytes, {remaining} remain"
    )]
    UnexpectedEof {
        section: Section,
        offset: usize,
        needed: usize,
        remaining: usize,
    },

    #[error("unterminated string in {section} at byte {offset}")]
    UnterminatedString { section: Section, offset: usize },

    #[error("integer overflow while calculating {context}")]
    ArithmeticOverflow { context: &'static str },

    #[error("invalid entry terminator 0x{value:04x} for `{path}` at tree byte {offset}")]
    InvalidEntryTerminator {
        path: String,
        offset: usize,
        value: u16,
    },

    #[error("invalid {section} size {size}: {reason}")]
    InvalidSectionSize {
        section: Section,
        size: u64,
        reason: &'static str,
    },

    #[error("{section} range {offset}..{end} exceeds the directory VPK size {file_size}")]
    SectionOutOfBounds {
        section: Section,
        offset: u64,
        end: u64,
        file_size: u64,
    },

    #[error("VPK path `{0}` does not have a usable file name")]
    InvalidArchivePath(PathBuf),

    #[error("VPK entry `{0}` was not found")]
    EntryNotFound(String),

    #[error("cannot reserve {size} bytes for {context}")]
    Allocation { context: &'static str, size: u64 },

    #[error(
        "entry `{path}` points outside `{archive}`: range {offset}..{end}, file size {file_size}"
    )]
    EntryOutOfBounds {
        path: String,
        archive: PathBuf,
        offset: u64,
        end: u64,
        file_size: u64,
    },

    #[error("CRC32 mismatch for `{path}`: expected 0x{expected:08x}, got 0x{actual:08x}")]
    EntryChecksumMismatch {
        path: String,
        expected: u32,
        actual: u32,
    },
}

impl Error {
    pub(crate) fn io(path: impl AsRef<Path>, source: io::Error) -> Self {
        Self::Io {
            path: path.as_ref().to_path_buf(),
            source,
        }
    }
}
