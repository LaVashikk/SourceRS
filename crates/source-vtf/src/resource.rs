use bitflags::bitflags;

use crate::cursor::Cursor;
use crate::subresource::image_data_len;
use crate::{Error, Header, Result, Section};

const RESOURCE_TABLE_OFFSET: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ResourceTag(u32);

impl ResourceTag {
    pub const THUMBNAIL_DATA: Self = Self(0x000001);
    pub const PALETTE_DATA: Self = Self(0x000002);
    pub const FALLBACK_DATA: Self = Self(0x000003);
    pub const PARTICLE_SHEET_DATA: Self = Self(0x000010);
    pub const HOTSPOT_DATA: Self = Self(0x00002b);
    pub const IMAGE_DATA: Self = Self(0x000030);
    pub const EXTENDED_FLAGS: Self = Self::from_bytes(*b"TS0");
    pub const CRC: Self = Self::from_bytes(*b"CRC");
    pub const AUX_COMPRESSION: Self = Self::from_bytes(*b"AXC");
    pub const LOD_CONTROL_INFO: Self = Self::from_bytes(*b"LOD");
    pub const KEY_VALUES_DATA: Self = Self::from_bytes(*b"KVD");
    pub const AUTHOR_INFO: Self = Self::from_bytes(*b"ATH");

    pub const fn from_bytes(bytes: [u8; 3]) -> Self {
        Self(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0]))
    }

    pub const fn value(self) -> u32 {
        self.0
    }

    pub const fn to_bytes(self) -> [u8; 3] {
        let bytes = self.0.to_le_bytes();
        [bytes[0], bytes[1], bytes[2]]
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct ResourceFlags: u8 {
        /// The four bytes in the directory entry are the payload itself, not an offset.
        const LOCAL_DATA = 1 << 1;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ResourceLocation {
    Inline([u8; 4]),
    Data { offset: usize, length: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Resource {
    tag: ResourceTag,
    flags: ResourceFlags,
    location: ResourceLocation,
}

impl Resource {
    pub const fn tag(&self) -> ResourceTag {
        self.tag
    }

    pub const fn flags(&self) -> ResourceFlags {
        self.flags
    }

    pub const fn location(&self) -> &ResourceLocation {
        &self.location
    }
}

pub(crate) fn parse(bytes: &[u8], header: &Header) -> Result<Vec<Resource>> {
    if !header.version().has_resource_table() {
        return legacy_resources(bytes, header);
    }

    let header_size =
        usize::try_from(header.header_size()).map_err(|_| Error::ArithmeticOverflow {
            context: "VTF header size",
        })?;
    let table = bytes
        .get(RESOURCE_TABLE_OFFSET..header_size)
        .ok_or(Error::RangeOutOfBounds {
            section: Section::ResourceTable,
            offset: RESOURCE_TABLE_OFFSET,
            end: header_size,
            file_size: bytes.len(),
        })?;
    let mut cursor = Cursor::new(table, Section::ResourceTable);
    let count =
        usize::try_from(header.resource_count()).map_err(|_| Error::ArithmeticOverflow {
            context: "VTF resource count",
        })?;
    let mut pending = Vec::new();
    pending
        .try_reserve_exact(count)
        .map_err(|_| Error::ResourceAllocation {
            count: header.resource_count(),
        })?;

    for _ in 0..count {
        let type_and_flags = cursor.read_u32()?;
        let tag = ResourceTag(type_and_flags & 0x00ff_ffff);
        let flags = ResourceFlags::from_bits_retain((type_and_flags >> 24) as u8);
        let value_bytes = cursor.read_u32()?.to_le_bytes();

        if pending
            .iter()
            .any(|entry: &PendingResource| entry.tag == tag)
        {
            return Err(Error::DuplicateResource { tag: tag.value() });
        }

        let value = if flags.contains(ResourceFlags::LOCAL_DATA) {
            PendingValue::Inline(value_bytes)
        } else {
            PendingValue::Offset(u32::from_le_bytes(value_bytes))
        };
        pending.push(PendingResource { tag, flags, value });
    }

    let mut offsets = pending
        .iter()
        .filter_map(|entry| match entry.value {
            PendingValue::Inline(_) => None,
            PendingValue::Offset(offset) => Some((usize::try_from(offset).ok(), entry.tag)),
        })
        .map(|(offset, tag)| {
            offset
                .ok_or(Error::ArithmeticOverflow {
                    context: "VTF resource offset",
                })
                .map(|offset| (offset, tag))
        })
        .collect::<Result<Vec<_>>>()?;
    offsets.sort_unstable_by_key(|&(offset, _)| offset);

    for pair in offsets.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(Error::DuplicateResourceOffset { offset: pair[0].0 });
        }
    }
    for &(offset, tag) in &offsets {
        if offset < header_size {
            return Err(Error::ResourceInsideHeader {
                tag: tag.value(),
                offset,
                header_size,
            });
        }
        if offset > bytes.len() {
            return Err(Error::RangeOutOfBounds {
                section: Section::ResourceData,
                offset,
                end: offset,
                file_size: bytes.len(),
            });
        }
    }

    let mut resources = Vec::new();
    resources
        .try_reserve_exact(count)
        .map_err(|_| Error::ResourceAllocation {
            count: header.resource_count(),
        })?;
    for entry in pending {
        let location = match entry.value {
            PendingValue::Inline(data) => ResourceLocation::Inline(data),
            PendingValue::Offset(raw_offset) => {
                let offset =
                    usize::try_from(raw_offset).map_err(|_| Error::ArithmeticOverflow {
                        context: "VTF resource offset",
                    })?;
                let position = offsets
                    .binary_search_by_key(&offset, |&(candidate, _)| candidate)
                    .map_err(|_| Error::MissingResource {
                        tag: entry.tag.value(),
                    })?;
                let end = offsets
                    .get(position + 1)
                    .map_or(bytes.len(), |&(next_offset, _)| next_offset);
                ResourceLocation::Data {
                    offset,
                    length: end - offset,
                }
            }
        };
        resources.push(Resource {
            tag: entry.tag,
            flags: entry.flags,
            location,
        });
    }

    validate_texture_resources(&resources, header)?;
    Ok(resources)
}

fn legacy_resources(bytes: &[u8], header: &Header) -> Result<Vec<Resource>> {
    let header_size =
        usize::try_from(header.header_size()).map_err(|_| Error::ArithmeticOverflow {
            context: "VTF header size",
        })?;
    let thumbnail_length = match header.thumbnail_format() {
        Some(format) => format.byte_len(
            u16::from(header.thumbnail_width()),
            u16::from(header.thumbnail_height()),
        )?,
        None => 0,
    };
    let image_offset =
        header_size
            .checked_add(thumbnail_length)
            .ok_or(Error::ArithmeticOverflow {
                context: "legacy VTF image offset",
            })?;
    ensure_range(
        Section::ThumbnailData,
        header_size,
        image_offset,
        bytes.len(),
    )?;

    let image_length = image_data_len(header)?;
    let image_end = image_offset
        .checked_add(image_length)
        .ok_or(Error::ArithmeticOverflow {
            context: "legacy VTF image end",
        })?;
    ensure_range(Section::ImageData, image_offset, image_end, bytes.len())?;

    let mut resources = Vec::with_capacity(2);
    if thumbnail_length != 0 {
        resources.push(Resource {
            tag: ResourceTag::THUMBNAIL_DATA,
            flags: ResourceFlags::empty(),
            location: ResourceLocation::Data {
                offset: header_size,
                length: thumbnail_length,
            },
        });
    }
    resources.push(Resource {
        tag: ResourceTag::IMAGE_DATA,
        flags: ResourceFlags::empty(),
        location: ResourceLocation::Data {
            offset: image_offset,
            length: image_length,
        },
    });
    Ok(resources)
}

fn validate_texture_resources(resources: &[Resource], header: &Header) -> Result<()> {
    let image = find(resources, ResourceTag::IMAGE_DATA).ok_or(Error::MissingResource {
        tag: ResourceTag::IMAGE_DATA.value(),
    })?;
    let image_length = image_data_len(header)?;
    validate_data_resource(image, image_length, Section::ImageData)?;

    let thumbnail_length = match header.thumbnail_format() {
        Some(format) => format.byte_len(
            u16::from(header.thumbnail_width()),
            u16::from(header.thumbnail_height()),
        )?,
        None => 0,
    };
    if thumbnail_length != 0 {
        let thumbnail =
            find(resources, ResourceTag::THUMBNAIL_DATA).ok_or(Error::MissingResource {
                tag: ResourceTag::THUMBNAIL_DATA.value(),
            })?;
        validate_data_resource(thumbnail, thumbnail_length, Section::ThumbnailData)?;
    }
    Ok(())
}

fn validate_data_resource(resource: &Resource, expected: usize, section: Section) -> Result<()> {
    match resource.location {
        ResourceLocation::Inline(_) => Err(Error::InlineTextureData {
            tag: resource.tag.value(),
        }),
        ResourceLocation::Data { length, .. } if length < expected => Err(Error::DataTooShort {
            section,
            expected,
            actual: length,
        }),
        ResourceLocation::Data { .. } => Ok(()),
    }
}

fn find(resources: &[Resource], tag: ResourceTag) -> Option<&Resource> {
    resources.iter().find(|resource| resource.tag == tag)
}

fn ensure_range(section: Section, offset: usize, end: usize, file_size: usize) -> Result<()> {
    if end > file_size {
        return Err(Error::RangeOutOfBounds {
            section,
            offset,
            end,
            file_size,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct PendingResource {
    tag: ResourceTag,
    flags: ResourceFlags,
    value: PendingValue,
}

#[derive(Debug, Clone, Copy)]
enum PendingValue {
    Inline([u8; 4]),
    Offset(u32),
}
