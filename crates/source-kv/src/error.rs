use std::fmt::Display;
use serde::{de, ser};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Message(String),

    #[error("Expected string")]
    ExpectedString,

    #[error("Expected key")]
    ExpectedKey,

    #[error("Syntax error at line {line}, column {column}: {msg}")]
    Syntax {
        line: usize,
        column: usize,
        msg: String,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}
