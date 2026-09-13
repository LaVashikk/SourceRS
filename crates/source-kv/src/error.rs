use std::fmt::Display;
use serde::{de, ser};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Message: {0}")]
    Message(String),

    #[error("Eof")]
    Eof,

    #[error("Expected string")]
    ExpectedString,

    #[error("Expected object start '{{'")]
    ExpectedObjectStart,

    #[error("Expected object end '}}'")]
    ExpectedObjectEnd,

    #[error("Expected key")]
    ExpectedKey,

    #[error("Expected value")]
    ExpectedValue,

    #[error("Trailing characters")]
    TrailingCharacters,

    #[error("Syntax error at line {line}, column {column}: {msg}")]
    Syntax {
        line: usize,
        column: usize,
        msg: String,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Float parse error: {0}")]
    FloatParse(#[from] std::num::ParseFloatError),

    #[error("Int parse error: {0}")]
    IntParse(#[from] std::num::ParseIntError),
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
