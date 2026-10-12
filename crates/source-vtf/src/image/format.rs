use num_enum::TryFromPrimitive;

use crate::{Error, Result, Version};

/// Image formats, numbered as in VTF 7.5
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TryFromPrimitive)]
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

impl ImageFormat {
    pub const fn is_block_compressed(self) -> bool {
        self.block().0 == 4
    }

    /// Bytes of one `width` x `height` image, block-compressed formats rounded up to whole 4x4 blocks
    pub const fn byte_len(self, width: u16, height: u16) -> usize {
        let (block, bytes) = self.block();
        (width as usize)
            .div_ceil(block)
            .saturating_mul((height as usize).div_ceil(block))
            .saturating_mul(bytes)
    }

    pub const fn has_alpha(self) -> bool {
        matches!(
            self,
            Self::Rgba8888
                | Self::Abgr8888
                | Self::Argb8888
                | Self::Bgra8888
                | Self::Dxt1OneBitAlpha
                | Self::Dxt3
                | Self::Dxt5
                | Self::Bgra4444
                | Self::Bgra5551
                | Self::Ia88
                | Self::A8
                | Self::Rgba16161616F
                | Self::Rgba16161616
                | Self::Rgba32323232F
                | Self::Rgba1010102
                | Self::Bgra1010102
        )
    }

    /// Side of a block in pixels and its size in bytes, uncompressed formats are 1x1 blocks
    const fn block(self) -> (usize, usize) {
        match self {
            Self::Dxt1 | Self::Dxt1OneBitAlpha | Self::Ati1n => (4, 8),
            Self::Dxt3 | Self::Dxt5 | Self::Ati2n => (4, 16),
            Self::Rgba32323232F => (1, 16),
            Self::Rgb323232F => (1, 12),
            Self::Rgba16161616F | Self::Rgba16161616 | Self::Rg3232F => (1, 8),
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
            | Self::Bgra1010102 => (1, 4),
            Self::Rgb888 | Self::Bgr888 | Self::Rgb888Bluescreen | Self::Bgr888Bluescreen => (1, 3),
            Self::Rgb565
            | Self::Ia88
            | Self::Bgr565
            | Self::Bgrx5551
            | Self::Bgra4444
            | Self::Bgra5551
            | Self::Uv88
            | Self::R16F => (1, 2),
            Self::I8 | Self::P8 | Self::A8 => (1, 1),
            Self::Empty => (1, 0),
        }
    }

    /// Reads a format ID the way a file of `version` numbers them
    pub(crate) fn from_wire(id: i32, version: Version, field: &'static str) -> Result<Self> {
        let id = if version < Version::V7_5 {
            // Before 7.5 it's SDK 2013's imageformat.h: 30..35 are NV/ATI depth formats, NV_NULL/ATI2N/ATI1N sit at 36..38
            // 7.5 put RG1616F, RG3232F, RGBX8888 at 30..32 and moved those three down!
            // todo: hmmmm, this keys the enum on the FILE version like 'SourcePP', but prob it's the engine branch that decides? check find and fix
            match id {
                30..=35 => return Err(Error::UnsupportedFormat { field, id }),
                36..=38 => id - 3,
                _ => id,
            }
        } else {
            id
        };

        Self::try_from(id).map_err(|_| Error::InvalidField {
            field,
            value: id.into(),
            reason: "unknown image format",
        })
    }
}
