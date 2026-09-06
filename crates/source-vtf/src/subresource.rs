use crate::{Error, Header, ImageFormat, Result};

#[derive(Debug, Clone, Copy)]
pub struct RawImage<'a> {
    format: ImageFormat,
    width: u16,
    height: u16,
    data: &'a [u8],
}

impl<'a> RawImage<'a> {
    pub(crate) const fn new(format: ImageFormat, width: u16, height: u16, data: &'a [u8]) -> Self {
        Self {
            format,
            width,
            height,
            data,
        }
    }

    pub const fn format(&self) -> ImageFormat {
        self.format
    }

    pub const fn width(&self) -> u16 {
        self.width
    }

    pub const fn height(&self) -> u16 {
        self.height
    }

    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    pub const fn len(&self) -> usize {
        self.data.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

pub(crate) fn image_data_len(header: &Header) -> Result<usize> {
    let mut total = 0usize;
    for mip in 0..header.mip_count() {
        let (width, height, depth) = mip_dimensions(header, mip);
        let one_slice = header.image_format().byte_len(width, height)?;
        let mip_length = checked_mul(one_slice, usize::from(depth), "mip depth byte length")?;
        let mip_length = checked_mul(
            mip_length,
            usize::from(header.face_count()),
            "mip face byte length",
        )?;
        let mip_length = checked_mul(
            mip_length,
            usize::from(header.frame_count()),
            "mip frame byte length",
        )?;
        total = total
            .checked_add(mip_length)
            .ok_or(Error::ArithmeticOverflow {
                context: "complete image data byte length",
            })?;
    }
    Ok(total)
}

pub(crate) fn locate(
    header: &Header,
    mip: u8,
    frame: u16,
    face: u8,
    slice: u16,
) -> Result<(usize, usize, u16, u16)> {
    check_index("mip", u32::from(mip), u32::from(header.mip_count()))?;
    check_index("frame", u32::from(frame), u32::from(header.frame_count()))?;
    check_index("face", u32::from(face), u32::from(header.face_count()))?;

    let (width, height, depth) = mip_dimensions(header, mip);
    check_index("depth slice", u32::from(slice), u32::from(depth))?;

    let mut offset = 0usize;

    // VTF stores API mip 0 last: all physically smaller mips precede the
    // requested mip, then frame/face/slice are tightly packed within it.
    for previous_mip in (mip + 1)..header.mip_count() {
        let (previous_width, previous_height, previous_depth) =
            mip_dimensions(header, previous_mip);
        let one_slice = header
            .image_format()
            .byte_len(previous_width, previous_height)?;
        let mip_length = checked_mul(
            one_slice,
            usize::from(previous_depth),
            "preceding mip depth byte length",
        )?;
        let mip_length = checked_mul(
            mip_length,
            usize::from(header.face_count()),
            "preceding mip face byte length",
        )?;
        let mip_length = checked_mul(
            mip_length,
            usize::from(header.frame_count()),
            "preceding mip frame byte length",
        )?;
        offset = offset
            .checked_add(mip_length)
            .ok_or(Error::ArithmeticOverflow {
                context: "subresource mip offset",
            })?;
    }

    let one_slice = header.image_format().byte_len(width, height)?;
    let frame_index = checked_mul(
        usize::from(frame),
        usize::from(header.face_count()),
        "subresource frame index",
    )?;
    let face_index =
        frame_index
            .checked_add(usize::from(face))
            .ok_or(Error::ArithmeticOverflow {
                context: "subresource face index",
            })?;
    let slice_index = checked_mul(face_index, usize::from(depth), "subresource depth index")?
        .checked_add(usize::from(slice))
        .ok_or(Error::ArithmeticOverflow {
            context: "subresource slice index",
        })?;
    offset = offset
        .checked_add(checked_mul(
            slice_index,
            one_slice,
            "subresource byte offset",
        )?)
        .ok_or(Error::ArithmeticOverflow {
            context: "subresource byte offset",
        })?;

    Ok((offset, one_slice, width, height))
}

pub(crate) fn mip_dimensions(header: &Header, mip: u8) -> (u16, u16, u16) {
    (
        mip_dimension(header.width(), mip),
        mip_dimension(header.height(), mip),
        mip_dimension(header.depth(), mip),
    )
}

fn mip_dimension(base: u16, mip: u8) -> u16 {
    base.checked_shr(u32::from(mip)).unwrap_or(0).max(1)
}

fn checked_mul(left: usize, right: usize, context: &'static str) -> Result<usize> {
    left.checked_mul(right)
        .ok_or(Error::ArithmeticOverflow { context })
}

fn check_index(axis: &'static str, index: u32, count: u32) -> Result<()> {
    if index >= count {
        return Err(Error::SubresourceOutOfRange { axis, index, count });
    }
    Ok(())
}
