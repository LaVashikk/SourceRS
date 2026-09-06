use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::header::Version;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, IntoPrimitive, TryFromPrimitive)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(i32)]
pub enum ImageFormat {
    Rgba8888 = 0,
    Abgr8888 = 1,
    Rgb888 = 2,
    Bgr888 = 3,
    Rgb565 = 4,
    I8 = 5,
    Ia88 = 6,
    P8 = 7,
    A8 = 8,
    Rgb888Bluescreen = 9,
    Bgr888Bluescreen = 10,
    Argb8888 = 11,
    Bgra8888 = 12,
    Dxt1 = 13,
    Dxt3 = 14,
    Dxt5 = 15,
    Bgrx8888 = 16,
    Bgr565 = 17,
    Bgrx5551 = 18,
    Bgra4444 = 19,
    Dxt1OneBitAlpha = 20,
    Bgra5551 = 21,
    Uv88 = 22,
    Uvwq8888 = 23,
    Rgba16161616F = 24,
    Rgba16161616 = 25,
    Uvlx8888 = 26,
    R32F = 27,
    Rgb323232F = 28,
    Rgba32323232F = 29,
    Rg1616F = 30,
    Rg3232F = 31,
    Rgbx8888 = 32,
    Empty = 33,
    Ati2n = 34,
    Ati1n = 35,
    Rgba1010102 = 36,
    Bgra1010102 = 37,
    R16F = 38,
}

#[derive(Debug, Clone, Copy)]
enum Storage {
    BytesPerPixel(usize),
    Block { bytes: usize },
    Empty,
}

impl ImageFormat {
    /// Numeric ID in the VTF 7.5 image-format table.
    pub const fn id(self) -> i32 {
        self as i32
    }

    pub const fn is_block_compressed(self) -> bool {
        matches!(self.storage(), Storage::Block { .. })
    }

    /// Checked byte length of one two-dimensional image.
    pub fn byte_len(self, width: u16, height: u16) -> Result<usize> {
        let width = usize::from(width);
        let height = usize::from(height);

        match self.storage() {
            Storage::BytesPerPixel(bytes) => width
                .checked_mul(height)
                .and_then(|pixels| pixels.checked_mul(bytes))
                .ok_or(Error::ArithmeticOverflow {
                    context: "uncompressed image byte length",
                }),
            Storage::Block { bytes } => {
                let blocks_wide = width.checked_add(3).ok_or(Error::ArithmeticOverflow {
                    context: "compressed image block width",
                })? / 4;
                let blocks_high = height.checked_add(3).ok_or(Error::ArithmeticOverflow {
                    context: "compressed image block height",
                })? / 4;
                blocks_wide
                    .checked_mul(blocks_high)
                    .and_then(|blocks| blocks.checked_mul(bytes))
                    .ok_or(Error::ArithmeticOverflow {
                        context: "compressed image byte length",
                    })
            }
            Storage::Empty => Ok(0),
        }
    }

    pub(crate) fn from_wire(value: i32, version: Version, field: &'static str) -> Result<Self> {
        let value = if version < Version::V7_5 {
            match value {
                33..=35 => {
                    return Err(Error::UnsupportedImageFormat {
                        value,
                        major: version.major(),
                        minor: version.minor(),
                        reason: "legacy depth-buffer formats are not valid raw image payloads",
                    });
                }
                36..=38 => value - 3,
                other => other,
            }
        } else {
            value
        };

        Self::try_from(value).map_err(|_| Error::UnknownImageFormat {
            field,
            value,
            major: version.major(),
            minor: version.minor(),
        })
    }

    const fn storage(self) -> Storage {
        match self {
            Self::Rgba8888
            | Self::Abgr8888
            | Self::Argb8888
            | Self::Bgra8888
            | Self::Bgrx8888
            | Self::Uvwq8888
            | Self::Uvlx8888
            | Self::R32F
            | Self::Rg1616F
            | Self::Rgbx8888
            | Self::Rgba1010102
            | Self::Bgra1010102 => Storage::BytesPerPixel(4),
            Self::Rgb888 | Self::Bgr888 | Self::Rgb888Bluescreen | Self::Bgr888Bluescreen => {
                Storage::BytesPerPixel(3)
            }
            Self::Rgb565
            | Self::Ia88
            | Self::Bgr565
            | Self::Bgrx5551
            | Self::Bgra4444
            | Self::Bgra5551
            | Self::Uv88
            | Self::R16F => Storage::BytesPerPixel(2),
            Self::I8 | Self::P8 | Self::A8 => Storage::BytesPerPixel(1),
            Self::Rgba16161616F | Self::Rgba16161616 | Self::Rg3232F => Storage::BytesPerPixel(8),
            Self::Rgb323232F => Storage::BytesPerPixel(12),
            Self::Rgba32323232F => Storage::BytesPerPixel(16),
            Self::Dxt1 | Self::Dxt1OneBitAlpha | Self::Ati1n => Storage::Block { bytes: 8 },
            Self::Dxt3 | Self::Dxt5 | Self::Ati2n => Storage::Block { bytes: 16 },
            Self::Empty => Storage::Empty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_compressed_images_to_complete_blocks() {
        assert_eq!(ImageFormat::Dxt1.byte_len(1, 1).unwrap(), 8);
        assert_eq!(ImageFormat::Dxt1.byte_len(5, 7).unwrap(), 32);
        assert_eq!(ImageFormat::Dxt5.byte_len(5, 7).unwrap(), 64);
    }

    #[test]
    fn translates_the_pre_75_tail_of_the_format_enum() {
        assert_eq!(
            ImageFormat::from_wire(36, Version::V7_4, "image").unwrap(),
            ImageFormat::Empty
        );
        assert_eq!(
            ImageFormat::from_wire(37, Version::V7_4, "image").unwrap(),
            ImageFormat::Ati2n
        );
        assert_eq!(
            ImageFormat::from_wire(38, Version::V7_4, "image").unwrap(),
            ImageFormat::Ati1n
        );
    }
}
