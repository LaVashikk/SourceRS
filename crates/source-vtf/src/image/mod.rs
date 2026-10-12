mod format;
pub(crate) mod layout;

pub use format::ImageFormat;

/// One 2D image: a mip of one frame, face and depth slice, or the thumbnail
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawImage<'a> {
    pub format: ImageFormat,
    pub width: u16,
    pub height: u16,
    /// Undecoded and tightly packed, block-compressed formats in whole 4x4 blocks
    pub data: &'a [u8],
}
