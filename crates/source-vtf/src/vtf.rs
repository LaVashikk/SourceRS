use std::borrow::Cow;
use std::fs;
use std::path::Path;

use crate::resource;
use crate::subresource::{self, RawImage};
use crate::{Error, Header, Resource, ResourceLocation, ResourceTag, Result};

#[derive(Debug)]
pub struct Vtf<'a> {
    bytes: Cow<'a, [u8]>,
    header: Header,
    resources: Vec<Resource>,
}

impl<'a> Vtf<'a> {
    /// Parses a VTF without copying its file data.
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self> {
        Self::parse(Cow::Borrowed(bytes))
    }

    pub const fn header(&self) -> &Header {
        &self.header
    }

    pub fn resources(&self) -> &[Resource] {
        &self.resources
    }

    pub fn resource(&self, tag: ResourceTag) -> Option<&Resource> {
        self.resources.iter().find(|resource| resource.tag() == tag)
    }

    pub fn resource_data(&self, tag: ResourceTag) -> Option<&[u8]> {
        let resource = self.resource(tag)?;
        match resource.location() {
            ResourceLocation::Inline(data) => Some(data),
            ResourceLocation::Data { offset, length } => {
                self.bytes.get(*offset..offset.checked_add(*length)?)
            }
        }
    }

    pub fn thumbnail(&self) -> Option<RawImage<'_>> {
        let format = self.header.thumbnail_format()?;
        let width = u16::from(self.header.thumbnail_width());
        let height = u16::from(self.header.thumbnail_height());
        if width == 0 || height == 0 {
            return None;
        }
        let expected = format.byte_len(width, height).ok()?;
        let data = self.resource_data(ResourceTag::THUMBNAIL_DATA)?;
        Some(RawImage::new(format, width, height, data.get(..expected)?))
    }

    pub const fn mip_count(&self) -> u8 {
        self.header.mip_count()
    }

    pub const fn frame_count(&self) -> u16 {
        self.header.frame_count()
    }

    pub const fn face_count(&self) -> u8 {
        self.header.face_count()
    }

    pub const fn depth(&self) -> u16 {
        self.header.depth()
    }

    pub fn mip_dimensions(&self, mip: u8) -> Result<(u16, u16, u16)> {
        if mip >= self.header.mip_count() {
            return Err(Error::SubresourceOutOfRange {
                axis: "mip",
                index: u32::from(mip),
                count: u32::from(self.header.mip_count()),
            });
        }
        Ok(subresource::mip_dimensions(&self.header, mip))
    }

    /// Returns one tightly packed two-dimensional image from the high-resolution payload.
    pub fn subresource(&self, mip: u8, frame: u16, face: u8, slice: u16) -> Result<RawImage<'_>> {
        let (relative_offset, length, width, height) =
            subresource::locate(&self.header, mip, frame, face, slice)?;
        let image = self
            .resource(ResourceTag::IMAGE_DATA)
            .ok_or(Error::MissingResource {
                tag: ResourceTag::IMAGE_DATA.value(),
            })?;
        let ResourceLocation::Data { offset, .. } = *image.location() else {
            return Err(Error::InlineTextureData {
                tag: ResourceTag::IMAGE_DATA.value(),
            });
        };
        let start = offset
            .checked_add(relative_offset)
            .ok_or(Error::ArithmeticOverflow {
                context: "subresource file offset",
            })?;
        let end = start.checked_add(length).ok_or(Error::ArithmeticOverflow {
            context: "subresource file end",
        })?;
        let data = self.bytes.get(start..end).ok_or(Error::RangeOutOfBounds {
            section: crate::Section::ImageData,
            offset: start,
            end,
            file_size: self.bytes.len(),
        })?;
        Ok(RawImage::new(
            self.header.image_format(),
            width,
            height,
            data,
        ))
    }

    fn parse(bytes: Cow<'a, [u8]>) -> Result<Self> {
        let header = Header::parse(&bytes)?;
        let resources = resource::parse(&bytes, &header)?;
        Ok(Self {
            bytes,
            header,
            resources,
        })
    }
}

impl Vtf<'static> {
    /// Reads a complete VTF into memory and parses it.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|error| Error::io(path, error))?;
        Self::parse(Cow::Owned(bytes))
    }
}
