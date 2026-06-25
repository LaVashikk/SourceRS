mod v1;
mod v2;

use crate::cursor::Cursor;
use crate::{Error, Result, Section};

pub(crate) const SIGNATURE: u32 = 0x55aa_1234;
pub(crate) const DIRECTORY_ARCHIVE_INDEX: u16 = 0x7fff;
pub(crate) const ENTRY_TERMINATOR: u16 = 0xffff;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Version {
    V1,
    V2,
}

impl Version {
    pub const fn number(self) -> u32 {
        match self {
            Self::V1 => 1,
            Self::V2 => 2,
        }
    }
}

impl TryFrom<u32> for Version {
    type Error = Error;

    fn try_from(value: u32) -> Result<Self> {
        match value {
            1 => Ok(Self::V1),
            2 => Ok(Self::V2),
            other => Err(Error::UnsupportedVersion(other)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ArchiveMd5Entry {
    pub archive_index: u32,
    pub offset: u32,
    pub length: u32,
    pub checksum: [u8; 16],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Header {
    pub(crate) version: Version,
    pub(crate) tree_size: u32,
    pub(crate) v2: Option<v2::HeaderV2>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Layout {
    pub(crate) tree_offset: u64,
    pub(crate) tree_size: u64,
    pub(crate) file_data_offset: u64,
    pub(crate) file_data_size: Option<u64>,
    pub(crate) archive_md5_offset: Option<u64>,
    pub(crate) archive_md5_size: u64,
    pub(crate) other_md5_offset: Option<u64>,
    pub(crate) other_md5_size: u64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FooterChecksums {
    pub(crate) tree: [u8; 16],
    pub(crate) archive_md5_section: [u8; 16],
    pub(crate) whole_file: [u8; 16],
}

pub(crate) fn parse_header(bytes: &[u8]) -> Result<Header> {
    let mut cursor = Cursor::new(bytes, Section::Header);
    let base = v1::HeaderV1::read(&mut cursor)?;

    if base.signature != SIGNATURE {
        return Err(Error::InvalidSignature(base.signature));
    }

    let version = Version::try_from(base.version)?;
    let v2 = match version {
        Version::V1 => None,
        Version::V2 => Some(v2::HeaderV2::read(&mut cursor)?),
    };

    Ok(Header {
        version,
        tree_size: base.tree_size,
        v2,
    })
}

impl Header {
    pub(crate) fn layout(self, file_size: u64) -> Result<Layout> {
        match self.v2 {
            Some(header) => header.layout(self.tree_size, file_size),
            None => v1::layout(self.tree_size, file_size),
        }
    }
}

pub(crate) use v2::{parse_archive_md5_entries, parse_footer_checksums};
