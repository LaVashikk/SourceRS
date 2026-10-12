use crate::cursor::range;
use crate::image::layout;
use crate::{Error, Header, Result};

const DICTIONARY_OFFSET: usize = 80; // sizeof(VTFFileHeaderV7_3_t)
const ENTRY_SIZE: usize = 8; // ResourceEntryInfo
/// RSRCF_HAS_NO_DATA_CHUNK: the entry's own 4 bytes are the data
const NO_DATA_CHUNK: u32 = 0x02 << 24;

/// Resource type ID, the low 24 bits of a dictionary entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ResourceTag(pub u32);

impl ResourceTag {
    pub const THUMBNAIL_DATA: Self = Self(0x01);
    pub const IMAGE_DATA: Self = Self(0x30);
    pub const PARTICLE_SHEET_DATA: Self = Self(0x10);
    pub const EXTENDED_FLAGS: Self = Self::from_bytes(*b"TS0");
    pub const CRC: Self = Self::from_bytes(*b"CRC");
    pub const LOD_CONTROL_INFO: Self = Self::from_bytes(*b"LOD");
    pub const STREAM_SETTINGS: Self = Self::from_bytes(*b"STR");
    /// Not in the SDK, VTFLib writes it
    pub const KEY_VALUES_DATA: Self = Self::from_bytes(*b"KVD");

    pub const fn from_bytes(bytes: [u8; 3]) -> Self {
        Self(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0]))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ResourceLocation {
    /// Stored in the dictionary entry itself
    Inline([u8; 4]),
    /// A byte range of the file
    Data { offset: usize, length: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Resource {
    pub tag: ResourceTag,
    pub location: ResourceLocation,
}

/// The 7.3+ resource dictionary, one entry per resource right after the fixed header
pub(super) fn parse_dictionary(bytes: &[u8], header: &Header, count: usize) -> Result<Vec<Resource>> {
    let table = range(bytes, DICTIONARY_OFFSET, count.saturating_mul(ENTRY_SIZE), "resource table")?;
    let mut resources = Vec::with_capacity(count);

    for entry in table.chunks_exact(ENTRY_SIZE) {
        let kind = u32::from_le_bytes(entry[..4].try_into().unwrap());
        let data: [u8; 4] = entry[4..].try_into().unwrap();
        let tag = ResourceTag(kind & 0x00ff_ffff);
        if kind & NO_DATA_CHUNK != 0 {
            resources.push(Resource { tag, location: ResourceLocation::Inline(data) });
            continue;
        }

        let offset = u32::from_le_bytes(data) as usize;
        let (offset, length) = match tag {
            ResourceTag::IMAGE_DATA => (offset, layout::image_len(header)),
            ResourceTag::THUMBNAIL_DATA => (offset, layout::thumbnail_len(header)),
            // Only image and thumbnail sizes come from the header, everything else starts with its own u32 size
            // The gap after it is vtex padding the next resource to 16 bytes with 0xDEADBEEF, lol

            // and todo: a broken sheet or KVD kills the whole texture right now, should cost only that resource prob
            _ => {
                let size = range(bytes, offset, 4, "resource size")?;
                (offset + 4, u32::from_le_bytes(size.try_into().unwrap()) as usize)
            }
        };

        range(bytes, offset, length, "resource data")?;

        resources.push(Resource { tag, location: ResourceLocation::Data { offset, length } });
    }

    require(&resources, ResourceTag::IMAGE_DATA)?;
    if layout::thumbnail_len(header) != 0 {
        require(&resources, ResourceTag::THUMBNAIL_DATA)?;
    }
    Ok(resources)
}

/// 7.0-7.2 have no dictionary: the thumbnail follows the header and the image follows the thumbnail
pub(super) fn legacy_resources(bytes: &[u8], header: &Header, header_size: usize) -> Result<Vec<Resource>> {
    let thumbnail_length = layout::thumbnail_len(header);
    let image_offset = header_size + thumbnail_length;
    let image_length = layout::image_len(header);
    range(bytes, image_offset, image_length, "image data")?;

    let mut resources = Vec::with_capacity(2);
    if thumbnail_length != 0 {
        resources.push(Resource {
            tag: ResourceTag::THUMBNAIL_DATA,
            location: ResourceLocation::Data { offset: header_size, length: thumbnail_length },
        });
    }

    resources.push(Resource {
        tag: ResourceTag::IMAGE_DATA,
        location: ResourceLocation::Data { offset: image_offset, length: image_length },
    });

    Ok(resources)
}

fn require(resources: &[Resource], tag: ResourceTag) -> Result<()> {
    match resources
        .iter()
        .find(|resource| resource.tag == tag)
    {
        Some(Resource { location: ResourceLocation::Data { .. }, .. }) => Ok(()),
        _ => Err(Error::MissingResource(tag)),
    }
}
