use crate::cursor::Cursor;
use crate::format::Layout;
use crate::{Error, Result, Section};

pub(crate) const HEADER_SIZE: u64 = 12;

#[derive(Debug, Clone, Copy)]
pub(crate) struct HeaderV1 {
    pub(crate) signature: u32,
    pub(crate) version: u32,
    pub(crate) tree_size: u32,
}

impl HeaderV1 {
    pub(crate) fn read(cursor: &mut Cursor<'_>) -> Result<Self> {
        Ok(Self {
            signature: cursor.read_u32()?,
            version: cursor.read_u32()?,
            tree_size: cursor.read_u32()?,
        })
    }
}

pub(crate) fn layout(tree_size: u32, file_size: u64) -> Result<Layout> {
    let tree_size = u64::from(tree_size);
    let file_data_offset = HEADER_SIZE
        .checked_add(tree_size)
        .ok_or(Error::ArithmeticOverflow {
            context: "VPK v1 tree end",
        })?;

    if file_data_offset > file_size {
        return Err(Error::SectionOutOfBounds {
            section: Section::Tree,
            offset: HEADER_SIZE,
            end: file_data_offset,
            file_size,
        });
    }

    Ok(Layout {
        tree_offset: HEADER_SIZE,
        tree_size,
        file_data_offset,
        file_data_size: None,
        archive_md5_offset: None,
        archive_md5_size: 0,
        other_md5_offset: None,
        other_md5_size: 0,
    })
}
