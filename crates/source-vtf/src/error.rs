use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Section {
    Header,
    ResourceTable,
    ThumbnailData,
    ImageData,
    ResourceData,
}

impl std::fmt::Display for Section {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Header => "header",
            Self::ResourceTable => "resource table",
            Self::ThumbnailData => "thumbnail data",
            Self::ImageData => "image data",
            Self::ResourceData => "resource data",
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

    #[error("invalid VTF signature 0x{0:08x}")]
    InvalidSignature(u32),

    #[error("unsupported VTF version {major}.{minor}")]
    UnsupportedVersion { major: u32, minor: u32 },

    #[error("invalid header size {declared} for VTF {major}.{minor}; expected {expected}")]
    InvalidHeaderSize {
        major: u32,
        minor: u32,
        declared: u32,
        expected: u32,
    },

    #[error(
        "unexpected end of {section} at byte {offset}: need {needed} bytes, {remaining} remain"
    )]
    UnexpectedEof {
        section: Section,
        offset: usize,
        needed: usize,
        remaining: usize,
    },

    #[error("integer overflow while calculating {context}")]
    ArithmeticOverflow { context: &'static str },

    #[error("invalid {field} value {value}: {reason}")]
    InvalidField {
        field: &'static str,
        value: u64,
        reason: &'static str,
    },

    #[error("unknown {field} image-format ID {value} in VTF {major}.{minor}")]
    UnknownImageFormat {
        field: &'static str,
        value: i32,
        major: u32,
        minor: u32,
    },

    #[error("image-format ID {value} in VTF {major}.{minor} is not supported: {reason}")]
    UnsupportedImageFormat {
        value: i32,
        major: u32,
        minor: u32,
        reason: &'static str,
    },

    #[error("VTF declares {count} resources; the safety limit is {limit}")]
    ResourceCountLimit { count: u32, limit: u32 },

    #[error("cannot reserve metadata for {count} VTF resources")]
    ResourceAllocation { count: u32 },

    #[error("resource 0x{tag:06x} is required but missing")]
    MissingResource { tag: u32 },

    #[error("resource 0x{tag:06x} occurs more than once")]
    DuplicateResource { tag: u32 },

    #[error("resource 0x{tag:06x} stores texture data inline in its four-byte directory slot")]
    InlineTextureData { tag: u32 },

    #[error("multiple non-inline resources point to byte {offset}")]
    DuplicateResourceOffset { offset: usize },

    #[error("resource 0x{tag:06x} points to byte {offset}, before header end {header_size}")]
    ResourceInsideHeader {
        tag: u32,
        offset: usize,
        header_size: usize,
    },

    #[error("{section} range {offset}..{end} exceeds the VTF size {file_size}")]
    RangeOutOfBounds {
        section: Section,
        offset: usize,
        end: usize,
        file_size: usize,
    },

    #[error("{section} is too short: expected at least {expected} bytes, found {actual}")]
    DataTooShort {
        section: Section,
        expected: usize,
        actual: usize,
    },

    #[error("{axis} index {index} is out of range for {count} values")]
    SubresourceOutOfRange {
        axis: &'static str,
        index: u32,
        count: u32,
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
