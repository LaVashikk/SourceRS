use std::borrow::Cow;
use std::fmt;
use std::path::Path;

use crate::cursor::range;
use crate::image::layout;
use crate::{Error, Header, OwnedVtf, RawImage, Resource, ResourceLocation, ResourceTag, Result, header};

/// A parsed VTF over borrowed or owned file bytes
#[derive(Clone, PartialEq)]
pub struct Vtf<'a> {
    pub header: Header,
    /// In dictionary order. 7.0-7.2 have no dictionary, there the thumbnail and image come from the fixed layout
    pub resources: Vec<Resource>,
    bytes: Cow<'a, [u8]>,
}

// todo: add clone for vtf!

impl<'a> Vtf<'a> {
    /// Parses a VTF without copying its file data
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self> {
        Self::parse(Cow::Borrowed(bytes))
    }

    /// Data of the first resource with `tag`: the 4 inline bytes, or its range of the file
    pub fn resource(&self, tag: ResourceTag) -> Option<&[u8]> {
        let resource = self
            .resources
            .iter()
            .find(|resource| resource.tag == tag)?;
        match &resource.location {
            ResourceLocation::Inline(data) => Some(data.as_slice()),
            ResourceLocation::Data { offset, length } => self
                .bytes
                .get(*offset..offset.saturating_add(*length)),
        }
    }

    pub fn thumbnail(&self) -> Option<RawImage<'_>> {
        let header = &self.header;
        if header.thumbnail_width == 0 || header.thumbnail_height == 0 {
            return None;
        }
        Some(RawImage {
            format: header.thumbnail_format?,
            width: header.thumbnail_width.into(),
            height: header.thumbnail_height.into(),
            data: self.resource(ResourceTag::THUMBNAIL_DATA)?,
        })
    }

    /// One mip of one frame, cubemap face and depth slice, `mip` 0 being the full-size image
    pub fn subresource(&self, mip: u8, frame: u16, face: u8, slice: u16) -> Result<RawImage<'_>> {
        let (offset, length, width, height) = layout::locate(&self.header, mip, frame, face, slice)?;
        let image = self
            .resource(ResourceTag::IMAGE_DATA)
            .ok_or(Error::MissingResource(ResourceTag::IMAGE_DATA))?;
        Ok(RawImage {
            format: self.header.image_format,
            width,
            height,
            data: range(image, offset, length, "subresource")?,
        })
    }

    /// The full-resolution image of the primary frame, cubemap face and volume slice
    pub fn base_image(&self) -> Result<RawImage<'_>> {
        self.subresource(0, 0, 0, 0)
    }

    // todo: add 'fn mip' helper?

    /// Converts a borrowed [`Vtf`] into [`OwnedVtf`], cloning the underlying bytes
    pub fn into_owned(self) -> OwnedVtf {
        Vtf {
            header: self.header,
            resources: self.resources,
            bytes: Cow::Owned(self.bytes.into_owned()),
        }
    }

    fn parse(bytes: Cow<'a, [u8]>) -> Result<Self> {
        let (header, resources) = header::parse(&bytes)?;
        Ok(Self { header, resources, bytes })
    }
}

impl OwnedVtf {
    /// Reads a whole VTF into memory and parses it
    pub fn open(path: impl AsRef<Path>) -> Result<OwnedVtf> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|source| Error::Io { path: path.to_owned(), source })?;
        Self::parse(Cow::Owned(bytes))
    }
}

// A derived Debug would dump the whole file through `bytes`
impl fmt::Debug for Vtf<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vtf")
            .field("header", &self.header)
            .field("resources", &self.resources)
            .finish_non_exhaustive()
    }
}
