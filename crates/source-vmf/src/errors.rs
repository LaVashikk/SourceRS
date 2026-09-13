//! This module defines the error types used in the VMF parser using `thiserror`.

use pest::error::Error as PestError;
use std::{io, num};
use thiserror::Error;

/// Represents an error that occurred during VMF parsing or processing.
#[derive(Error, Debug)]
pub enum VmfError {
    /// An I/O error occurred.
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// A parsing error occurred during the Pest parsing phase.
    #[error("VMF parse error: {0}")]
    Parse(#[from] Box<PestError<crate::parser::Rule>>),

    /// The VMF structure or content is invalid or unexpected.
    #[error("Invalid VMF format: {0}")]
    InvalidFormat(String),

    /// An error occurred while parsing an integer value for a specific key.
    #[error("Integer parse error for key '{key}': {source}")]
    ParseInt {
        key: String,
        #[source]
        source: num::ParseIntError,
    },

    /// An error occurred while parsing a float value for a specific key.
    #[error("Float parse error for key '{key}': {source}")]
    ParseFloat {
        key: String,
        #[source]
        source: num::ParseFloatError,
    },
}

/// A type alias for `Result` that uses `VmfError` as the error type.
pub type VmfResult<T> = Result<T, VmfError>;

impl From<(std::num::ParseIntError, String)> for VmfError {
    fn from(err: (std::num::ParseIntError, String)) -> Self {
        VmfError::ParseInt {
            source: err.0,
            key: err.1,
        }
    }
}

impl From<(std::num::ParseFloatError, String)> for VmfError {
    fn from(err: (std::num::ParseFloatError, String)) -> Self {
        VmfError::ParseFloat {
            source: err.0,
            key: err.1,
        }
    }
}
