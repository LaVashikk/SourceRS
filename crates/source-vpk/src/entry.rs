use crate::format::DIRECTORY_ARCHIVE_INDEX;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ArchiveIndex {
    Directory,
    External(u16),
}

impl ArchiveIndex {
    pub(crate) const fn from_raw(value: u16) -> Self {
        if value == DIRECTORY_ARCHIVE_INDEX {
            Self::Directory
        } else {
            Self::External(value)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Entry {
    crc32: u32,
    archive: ArchiveIndex,
    offset: u32,
    data_length: u32,
    preload: Vec<u8>,
}

impl Entry {
    pub(crate) fn new(
        crc32: u32,
        archive: ArchiveIndex,
        offset: u32,
        data_length: u32,
        preload: Vec<u8>,
    ) -> Self {
        Self {
            crc32,
            archive,
            offset,
            data_length,
            preload,
        }
    }

    pub const fn crc32(&self) -> u32 {
        self.crc32
    }

    pub const fn archive(&self) -> ArchiveIndex {
        self.archive
    }

    pub const fn offset(&self) -> u32 {
        self.offset
    }

    /// Number of bytes stored outside the directory tree's preload area.
    pub const fn data_length(&self) -> u32 {
        self.data_length
    }

    pub fn preload(&self) -> &[u8] {
        &self.preload
    }

    pub fn len(&self) -> u64 {
        self.preload.len() as u64 + u64::from(self.data_length)
    }

    pub fn is_empty(&self) -> bool {
        self.preload.is_empty() && self.data_length == 0
    }
}
