use crate::cursor::Cursor;
use crate::format::{ArchiveMd5Entry, FooterChecksums, Layout};
use crate::{Error, Result, Section};

pub(crate) const HEADER_SIZE: u64 = 28;
const ARCHIVE_MD5_ENTRY_SIZE: u64 = 28;
const FOOTER_CHECKSUMS_SIZE: u64 = 48;

#[derive(Debug, Clone, Copy)]
pub(crate) struct HeaderV2 {
    file_data_size: u32,
    archive_md5_size: u32,
    other_md5_size: u32,
    signature_size: u32,
}

impl HeaderV2 {
    pub(crate) fn read(cursor: &mut Cursor<'_>) -> Result<Self> {
        Ok(Self {
            file_data_size: cursor.read_u32()?,
            archive_md5_size: cursor.read_u32()?,
            other_md5_size: cursor.read_u32()?,
            signature_size: cursor.read_u32()?,
        })
    }

    pub(crate) fn layout(self, tree_size: u32, file_size: u64) -> Result<Layout> {
        let tree_size = u64::from(tree_size);
        let file_data_size = u64::from(self.file_data_size);
        let archive_md5_size = u64::from(self.archive_md5_size);
        let other_md5_size = u64::from(self.other_md5_size);
        let signature_size = u64::from(self.signature_size);

        if archive_md5_size % ARCHIVE_MD5_ENTRY_SIZE != 0 {
            return Err(Error::InvalidSectionSize {
                section: Section::ArchiveMd5,
                size: archive_md5_size,
                reason: "the size must be a multiple of 28 bytes",
            });
        }

        let file_data_offset = checked_add(HEADER_SIZE, tree_size, "VPK v2 tree end")?;
        let archive_md5_offset =
            checked_add(file_data_offset, file_data_size, "VPK v2 file data end")?;
        let other_md5_offset = checked_add(
            archive_md5_offset,
            archive_md5_size,
            "VPK v2 archive MD5 end",
        )?;
        let signature_offset =
            checked_add(other_md5_offset, other_md5_size, "VPK v2 other MD5 end")?;
        let end = checked_add(signature_offset, signature_size, "VPK v2 signature end")?;

        ensure_section(Section::Tree, HEADER_SIZE, file_data_offset, file_size)?;
        ensure_section(
            Section::FileData,
            file_data_offset,
            archive_md5_offset,
            file_size,
        )?;
        ensure_section(
            Section::ArchiveMd5,
            archive_md5_offset,
            other_md5_offset,
            file_size,
        )?;
        ensure_section(
            Section::OtherMd5,
            other_md5_offset,
            signature_offset,
            file_size,
        )?;
        ensure_section(Section::Signature, signature_offset, end, file_size)?;

        Ok(Layout {
            tree_offset: HEADER_SIZE,
            tree_size,
            file_data_offset,
            file_data_size: Some(file_data_size),
            archive_md5_offset: Some(archive_md5_offset),
            archive_md5_size,
            other_md5_offset: Some(other_md5_offset),
            other_md5_size,
        })
    }
}

pub(crate) fn parse_archive_md5_entries(bytes: &[u8]) -> Result<Vec<ArchiveMd5Entry>> {
    let mut cursor = Cursor::new(bytes, Section::ArchiveMd5);
    let mut entries = Vec::with_capacity(bytes.len() / ARCHIVE_MD5_ENTRY_SIZE as usize);

    while cursor.remaining() > 0 {
        entries.push(ArchiveMd5Entry {
            archive_index: cursor.read_u32()?,
            offset: cursor.read_u32()?,
            length: cursor.read_u32()?,
            checksum: read_checksum(&mut cursor)?,
        });
    }

    Ok(entries)
}

pub(crate) fn parse_footer_checksums(bytes: &[u8]) -> Result<FooterChecksums> {
    if bytes.len() != FOOTER_CHECKSUMS_SIZE as usize {
        return Err(Error::InvalidSectionSize {
            section: Section::OtherMd5,
            size: bytes.len() as u64,
            reason: "the standard checksum section is exactly 48 bytes",
        });
    }

    let mut cursor = Cursor::new(bytes, Section::OtherMd5);
    Ok(FooterChecksums {
        tree: read_checksum(&mut cursor)?,
        archive_md5_section: read_checksum(&mut cursor)?,
        whole_file: read_checksum(&mut cursor)?,
    })
}

fn read_checksum(cursor: &mut Cursor<'_>) -> Result<[u8; 16]> {
    let bytes = cursor.read_bytes(16)?;
    let mut checksum = [0; 16];
    checksum.copy_from_slice(bytes);
    Ok(checksum)
}

fn ensure_section(section: Section, offset: u64, end: u64, file_size: u64) -> Result<()> {
    if end > file_size {
        return Err(Error::SectionOutOfBounds {
            section,
            offset,
            end,
            file_size,
        });
    }
    Ok(())
}

fn checked_add(left: u64, right: u64, context: &'static str) -> Result<u64> {
    left.checked_add(right)
        .ok_or(Error::ArithmeticOverflow { context })
}
