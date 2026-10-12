mod flags;
mod resource;

pub use flags::TextureFlags;
pub use resource::{Resource, ResourceLocation, ResourceTag};

use crate::cursor::Cursor;
use crate::{Error, ImageFormat, Result};

const SIGNATURE: u32 = u32::from_le_bytes(*b"VTF\0");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Version {
    V7_0 = 0,
    V7_1 = 1,
    V7_2 = 2,
    V7_3 = 3,
    V7_4 = 4,
    V7_5 = 5,
}

impl Version {
    pub const fn minor(self) -> u32 {
        self as u32
    }

    pub const fn has_depth(self) -> bool {
        matches!(self, Self::V7_2 | Self::V7_3 | Self::V7_4 | Self::V7_5)
    }

    pub const fn has_resource_table(self) -> bool {
        matches!(self, Self::V7_3 | Self::V7_4 | Self::V7_5)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Header {
    pub version: Version,
    pub width: u16,
    pub height: u16,
    /// Slices of a volume texture, 1 for everything else
    pub depth: u16,
    pub flags: TextureFlags,
    pub frame_count: u16,
    pub start_frame: u16,
    pub reflectivity: [f32; 3],
    pub bump_map_scale: f32,
    pub image_format: ImageFormat,
    pub mip_count: u8,
    /// `None` when the VTF has no thumbnail
    pub thumbnail_format: Option<ImageFormat>,
    pub thumbnail_width: u8,
    pub thumbnail_height: u8,
}

impl Header {
    /// 1, or 6 for cubemaps, or 7 for cubemaps that also store a spheremap
    pub const fn face_count(&self) -> u8 {
        if !self.flags.contains(TextureFlags::ENVMAP) {
            return 1;
        }

        // Yes, 7 faces: 7.1-7.4 cubemaps carry a spheremap fallback (CUBEMAP_FACE_SPHEREMAP in vtf.h)
        // todo: 0xffff = no spheremap is VTFLib's rule, and VTFLib applies it to 7.0 too, we DON'T. the cool 'SourcePP' counts by data size instead
        match self.version {
            Version::V7_0 | Version::V7_5 => 6,
            _ if self.start_frame == u16::MAX => 6,
            _ => 7,
        }
    }

    /// Width, height and depth of a mip, mip 0 being the full size. Halved per mip, never below 1
    pub fn mip_dimensions(&self, mip: u8) -> (u16, u16, u16) {
        let halve = |size: u16| {
            size.checked_shr(u32::from(mip))
                .unwrap_or(0)
                .max(1)
        };
        (halve(self.width), halve(self.height), halve(self.depth))
    }
}

/// Reads the header and the resources it lists, since 7.3 the resource dictionary is part of the header
pub(crate) fn parse(bytes: &[u8]) -> Result<(Header, Vec<Resource>)> {
    // todo: this func piece of crap kinda, i dont like, but 'it just works'
    let mut c = Cursor::new(bytes, "header");
    let signature = c.read_u32()?;
    if signature != SIGNATURE {
        return Err(Error::InvalidSignature(signature));
    }

    let major = c.read_u32()?;
    let minor = c.read_u32()?;
    let version = match (major, minor) {
        (7, 0) => Version::V7_0,
        (7, 1) => Version::V7_1,
        (7, 2) => Version::V7_2,
        (7, 3) => Version::V7_3,
        (7, 4) => Version::V7_4,
        (7, 5) => Version::V7_5,
        _ => return Err(Error::UnsupportedVersion { major, minor }),
    };

    let header_size = c.read_u32()?;
    let width = c.read_u16()?;
    let height = c.read_u16()?;
    let flags = TextureFlags::from_bits_retain(c.read_u32()?);
    let frame_count = c.read_u16()?;
    let start_frame = c.read_u16()?;
    c.skip(4)?; // padding, reflectivity is a VectorAligned at 32

    let reflectivity = [c.read_f32()?, c.read_f32()?, c.read_f32()?];
    c.skip(4)?; // VectorAligned's w

    let bump_map_scale = c.read_f32()?;
    let image_format = ImageFormat::from_wire(c.read_i32()?, version, "image format")?;
    let mip_count = c.read_u8()?;
    let thumbnail_format = match c.read_i32()? {
        -1 => None,
        id => Some(ImageFormat::from_wire(id, version, "thumbnail format")?),
    };

    let thumbnail_width = c.read_u8()?;
    let thumbnail_height = c.read_u8()?;
    let depth = if version.has_depth() { c.read_u16()? } else { 1 };
    let resource_count = if version.has_resource_table() {
        c.skip(3)?; // pad4
        c.read_u32()?
    } else {
        0
    };

    let expected_size = match version {
        Version::V7_0 | Version::V7_1 => 64,
        Version::V7_2 => 80,
        // sizeof(VTFFileHeaderV7_3_t), then an 8-byte ResourceEntryInfo per resource
        _ => 80 + 8 * u64::from(resource_count),
    };

    if u64::from(header_size) != expected_size {
        return Err(Error::InvalidField {
            field: "header size",
            value: header_size.into(),
            reason: "doesn't match the version and resource count",
        });
    }

    for (field, value) in [
        ("width", width),
        ("height", height),
        ("depth", depth),
        ("frame count", frame_count),
        ("mip count", mip_count.into()),
    ] {
        if value == 0 {
            return Err(Error::InvalidField { field, value: 0, reason: "cannot be zero" });
        }
    }

    if thumbnail_format.is_none() && thumbnail_width != 0 && thumbnail_height != 0 {
        return Err(Error::InvalidField {
            field: "thumbnail format",
            value: -1,
            reason: "a thumbnail with a non-zero size needs a format",
        });
    }

    // Full size plus one mip per halving, zero sizes are rejected above so ilog2 can't panic
    if u32::from(mip_count) > width.max(height).max(depth).ilog2() + 1 {
        return Err(Error::InvalidField {
            field: "mip count",
            value: mip_count.into(),
            reason: "more mips than sizes down to 1x1",
        });
    }

    let header = Header {
        version,
        width,
        height,
        depth,
        flags,
        frame_count,
        start_frame,
        reflectivity,
        bump_map_scale,
        image_format,
        mip_count,
        thumbnail_format,
        thumbnail_width,
        thumbnail_height,
    };

    let resources = if version.has_resource_table() {
        resource::parse_dictionary(bytes, &header, resource_count as usize)?
    } else {
        resource::legacy_resources(bytes, &header, header_size as usize)?
    };

    Ok((header, resources))
}
