use std::io;
use std::path::PathBuf;

use thiserror::Error;

use crate::ResourceTag;

pub type Result<T> = std::result::Result<T, Error>;

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

    #[error("{field} {id} is a pre-7.5 depth-buffer format, which has no image data to read")]
    UnsupportedFormat { field: &'static str, id: i32 },

    #[error("VTF {context} range {offset}..{end} is outside 0..{size}")]
    OutOfBounds {
        context: &'static str,
        offset: usize,
        end: usize,
        size: usize,
    },

    #[error("invalid {field} value {value}: {reason}")]
    InvalidField {
        field: &'static str,
        value: i64,
        reason: &'static str,
    },

    /// Not in the resource dictionary, or listed inline without a data chunk
    #[error("VTF has no data for resource 0x{:06x}", .0.0)]
    MissingResource(ResourceTag),

    #[error("{context} index {index} is out of range for {count} values")]
    InvalidIndex {
        context: &'static str,
        index: u32,
        count: u32,
    },
}
