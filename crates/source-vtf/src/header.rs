use crate::cursor::Cursor;
use crate::{Error, ImageFormat, Result, Section, TextureFlags};

const SIGNATURE: u32 = u32::from_le_bytes(*b"VTF\0");
const LEGACY_HEADER_SIZE: u32 = 64;
const V7_2_HEADER_SIZE: u32 = 80;
const RESOURCE_HEADER_SIZE: u32 = 80;
const RESOURCE_ENTRY_SIZE: u32 = 8;
const MAX_RESOURCES: u32 = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Version {
    V7_0,
    V7_1,
    V7_2,
    V7_3,
    V7_4,
    V7_5,
}

impl Version {
    pub const fn major(self) -> u32 {
        7
    }

    pub const fn minor(self) -> u32 {
        match self {
            Self::V7_0 => 0,
            Self::V7_1 => 1,
            Self::V7_2 => 2,
            Self::V7_3 => 3,
            Self::V7_4 => 4,
            Self::V7_5 => 5,
        }
    }

    pub const fn has_depth(self) -> bool {
        matches!(self, Self::V7_2 | Self::V7_3 | Self::V7_4 | Self::V7_5)
    }

    pub const fn has_resource_table(self) -> bool {
        matches!(self, Self::V7_3 | Self::V7_4 | Self::V7_5)
    }
}

impl TryFrom<(u32, u32)> for Version {
    type Error = Error;

    fn try_from((major, minor): (u32, u32)) -> Result<Self> {
        match (major, minor) {
            (7, 0) => Ok(Self::V7_0),
            (7, 1) => Ok(Self::V7_1),
            (7, 2) => Ok(Self::V7_2),
            (7, 3) => Ok(Self::V7_3),
            (7, 4) => Ok(Self::V7_4),
            (7, 5) => Ok(Self::V7_5),
            _ => Err(Error::UnsupportedVersion { major, minor }),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Header {
    version: Version,
    header_size: u32,
    width: u16,
    height: u16,
    flags: TextureFlags,
    frame_count: u16,
    start_frame: u16,
    reflectivity: [f32; 3],
    bump_map_scale: f32,
    image_format_id: i32,
    image_format: ImageFormat,
    mip_count: u8,
    thumbnail_format_id: i32,
    thumbnail_format: Option<ImageFormat>,
    thumbnail_width: u8,
    thumbnail_height: u8,
    depth: u16,
    resource_count: u32,
}

impl Header {
    pub const fn version(&self) -> Version {
        self.version
    }

    pub const fn header_size(&self) -> u32 {
        self.header_size
    }

    pub const fn width(&self) -> u16 {
        self.width
    }

    pub const fn height(&self) -> u16 {
        self.height
    }

    pub const fn flags(&self) -> TextureFlags {
        self.flags
    }

    pub const fn frame_count(&self) -> u16 {
        self.frame_count
    }

    pub const fn start_frame(&self) -> u16 {
        self.start_frame
    }

    pub const fn reflectivity(&self) -> [f32; 3] {
        self.reflectivity
    }

    pub const fn bump_map_scale(&self) -> f32 {
        self.bump_map_scale
    }

    /// Raw image-format ID as it appeared in this VTF version's enum.
    pub const fn image_format_id(&self) -> i32 {
        self.image_format_id
    }

    /// Normalized image format. IDs 36–38 in VTF 7.0–7.4 are mapped to
    /// their VTF 7.5 names while [`Header::image_format_id`] remains lossless.
    pub const fn image_format(&self) -> ImageFormat {
        self.image_format
    }

    pub const fn mip_count(&self) -> u8 {
        self.mip_count
    }

    pub const fn thumbnail_format_id(&self) -> i32 {
        self.thumbnail_format_id
    }

    pub const fn thumbnail_format(&self) -> Option<ImageFormat> {
        self.thumbnail_format
    }

    pub const fn thumbnail_width(&self) -> u8 {
        self.thumbnail_width
    }

    pub const fn thumbnail_height(&self) -> u8 {
        self.thumbnail_height
    }

    pub const fn depth(&self) -> u16 {
        self.depth
    }

    pub const fn resource_count(&self) -> u32 {
        self.resource_count
    }

    /// Number of stored faces, including the legacy sphere-map face where the
    /// 7.1–7.4 header says it is present.
    pub const fn face_count(&self) -> u8 {
        if !self.flags.contains(TextureFlags::ENVMAP) {
            return 1;
        }
        match self.version {
            Version::V7_0 | Version::V7_5 => 6,
            Version::V7_1 | Version::V7_2 | Version::V7_3 | Version::V7_4 => {
                if self.start_frame == u16::MAX { 6 } else { 7 }
            }
        }
    }

    pub(crate) fn parse(bytes: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(bytes, Section::Header);
        let signature = cursor.read_u32()?;
        if signature != SIGNATURE {
            return Err(Error::InvalidSignature(signature));
        }

        let major = cursor.read_u32()?;
        let minor = cursor.read_u32()?;
        let version = Version::try_from((major, minor))?;
        let header_size = cursor.read_u32()?;
        let width = cursor.read_u16()?;
        let height = cursor.read_u16()?;
        let flags = TextureFlags::from_bits_retain(cursor.read_u32()?);
        let frame_count = cursor.read_u16()?;
        let start_frame = cursor.read_u16()?;
        cursor.skip(4)?;
        let reflectivity = [cursor.read_f32()?, cursor.read_f32()?, cursor.read_f32()?];
        cursor.skip(4)?;
        let bump_map_scale = cursor.read_f32()?;
        let image_format_id = cursor.read_i32()?;
        let image_format = ImageFormat::from_wire(image_format_id, version, "high-resolution")?;
        let mip_count = cursor.read_u8()?;
        let thumbnail_format_id = cursor.read_i32()?;
        let thumbnail_width = cursor.read_u8()?;
        let thumbnail_height = cursor.read_u8()?;
        let thumbnail_format = if thumbnail_format_id == -1 {
            None
        } else {
            Some(ImageFormat::from_wire(
                thumbnail_format_id,
                version,
                "low-resolution",
            )?)
        };
        let depth = if version.has_depth() {
            cursor.read_u16()?
        } else {
            1
        };

        let resource_count = if version.has_resource_table() {
            cursor.skip(3)?;
            let count = cursor.read_u32()?;
            cursor.skip(8)?;
            count
        } else {
            0
        };

        if resource_count > MAX_RESOURCES {
            return Err(Error::ResourceCountLimit {
                count: resource_count,
                limit: MAX_RESOURCES,
            });
        }

        let expected_header_size = match version {
            Version::V7_0 | Version::V7_1 => LEGACY_HEADER_SIZE,
            Version::V7_2 => V7_2_HEADER_SIZE,
            Version::V7_3 | Version::V7_4 | Version::V7_5 => RESOURCE_HEADER_SIZE
                .checked_add(resource_count.checked_mul(RESOURCE_ENTRY_SIZE).ok_or(
                    Error::ArithmeticOverflow {
                        context: "VTF resource table byte length",
                    },
                )?)
                .ok_or(Error::ArithmeticOverflow {
                    context: "VTF header byte length",
                })?,
        };

        if header_size != expected_header_size {
            return Err(Error::InvalidHeaderSize {
                major,
                minor,
                declared: header_size,
                expected: expected_header_size,
            });
        }

        let remaining_header = usize::try_from(header_size)
            .ok()
            .and_then(|size| size.checked_sub(cursor.position()))
            .ok_or(Error::InvalidHeaderSize {
                major,
                minor,
                declared: header_size,
                expected: expected_header_size,
            })?;
        cursor.skip(remaining_header)?;

        validate_nonzero("width", width)?;
        validate_nonzero("height", height)?;
        validate_nonzero("frame count", frame_count)?;
        validate_nonzero("mip count", mip_count)?;
        validate_nonzero("depth", depth)?;

        if (thumbnail_width != 0 && thumbnail_height != 0) && thumbnail_format.is_none() {
            return Err(Error::InvalidField {
                field: "low-resolution image format",
                value: thumbnail_format_id as u32 as u64,
                reason: "a thumbnail with non-zero dimensions needs a format",
            });
        }

        let maximum_mips = maximum_mip_count(width, height, depth);
        if mip_count > maximum_mips {
            return Err(Error::InvalidField {
                field: "mip count",
                value: u64::from(mip_count),
                reason: "it exceeds the number of distinct mip dimensions",
            });
        }

        Ok(Self {
            version,
            header_size,
            width,
            height,
            flags,
            frame_count,
            start_frame,
            reflectivity,
            bump_map_scale,
            image_format_id,
            image_format,
            mip_count,
            thumbnail_format_id,
            thumbnail_format,
            thumbnail_width,
            thumbnail_height,
            depth,
            resource_count,
        })
    }
}

fn validate_nonzero(field: &'static str, value: impl Into<u64> + Copy) -> Result<()> {
    if value.into() == 0 {
        return Err(Error::InvalidField {
            field,
            value: 0,
            reason: "zero is not valid",
        });
    }
    Ok(())
}

fn maximum_mip_count(width: u16, height: u16, depth: u16) -> u8 {
    let mut largest = width.max(height).max(depth);
    let mut count = 0;
    while largest != 0 {
        count += 1;
        largest >>= 1;
    }
    count
}
