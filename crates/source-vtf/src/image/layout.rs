//! Where each mip, frame, face and depth slice sits in the image data
//!
//! Sizes saturate instead of overflowing, so an absurd header ends up as a range past the end of the file

use std::ops::Range;

use crate::{Error, Header, Result};

/// The whole high-resolution image data
pub(crate) fn image_len(header: &Header) -> usize {
    mips_len(header, 0..header.mip_count)
}

pub(crate) fn thumbnail_len(header: &Header) -> usize {
    header
        .thumbnail_format
        .map_or(0, |format| format.byte_len(header.thumbnail_width.into(), header.thumbnail_height.into()))
}

pub(crate) fn locate(
    header: &Header,
    mip: u8,
    frame: u16,
    face: u8,
    slice: u16,
) -> Result<(usize, usize, u16, u16)> {
    let (width, height, depth) = header.mip_dimensions(mip);
    check("mip", mip, header.mip_count)?;
    check("frame", frame, header.frame_count)?;
    check("face", face, header.face_count())?;
    check("depth slice", slice, depth)?;

    // Valve's backwards logic: smallest mip FIRST, full resolution last
    // Prob made sense in 2004, low mips load without seeking past the big ones -_-
    // TODO: zero volume textures in 15k files, the depth slice math has only synthetic tests behind it
    let smaller_mips = mips_len(header, mip + 1..header.mip_count);

    // Inside a mip: frames, then faces, then depth slices
    let length = header.image_format.byte_len(width, height);
    let index = usize::from(frame)
        .saturating_mul(header.face_count().into())
        .saturating_add(face.into())
        .saturating_mul(depth.into())
        .saturating_add(slice.into());
    let offset = smaller_mips.saturating_add(index.saturating_mul(length));

    Ok((offset, length, width, height))
}

fn mips_len(header: &Header, mips: Range<u8>) -> usize {
    mips.fold(0usize, |total, mip| {
        let (width, height, depth) = header.mip_dimensions(mip);
        let mip_len = header
            .image_format
            .byte_len(width, height)
            .saturating_mul(depth.into())
            .saturating_mul(header.face_count().into())
            .saturating_mul(header.frame_count.into());
        total.saturating_add(mip_len)
    })
}

fn check<T: Into<u32>>(context: &'static str, index: T, count: T) -> Result<()> {
    let (index, count) = (index.into(), count.into());
    if index < count { Ok(()) } else { Err(Error::InvalidIndex { context, index, count }) }
}
