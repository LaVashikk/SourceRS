#![forbid(unsafe_code)]

//! [`Vpk`] parses the small directory index eagerly, while [`EntryReader`]
//! streams entry data from the directory file or its numbered archive. Preload
//! bytes are exposed as the beginning of the same stream.

mod archive;
mod cursor;
mod entry;
mod error;
mod format;
mod reader;
mod tree;

pub use archive::{ArchiveVerification, Vpk};
pub use entry::{ArchiveIndex, Entry};
pub use error::{Error, Result, Section};
pub use format::{ArchiveMd5Entry, Version};
pub use reader::EntryReader;
