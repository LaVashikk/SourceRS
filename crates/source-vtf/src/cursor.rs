use crate::{Error, Result};

/// Little-endian fields read one after another
pub(crate) struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
    context: &'static str,
}

impl<'a> Cursor<'a> {
    pub(crate) const fn new(bytes: &'a [u8], context: &'static str) -> Self {
        Self { bytes, position: 0, context }
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8> {
        Ok(self.take::<1>()?[0])
    }

    pub(crate) fn read_u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take()?))
    }

    pub(crate) fn read_u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take()?))
    }

    pub(crate) fn read_i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take()?))
    }

    pub(crate) fn read_f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.take()?))
    }

    pub(crate) fn skip(&mut self, length: usize) -> Result<()> {
        range(self.bytes, self.position, length, self.context)?;
        self.position += length;
        Ok(())
    }

    fn take<const N: usize>(&mut self) -> Result<[u8; N]> {
        let bytes = range(self.bytes, self.position, N, self.context)?;
        self.position += N;
        Ok(bytes.try_into().unwrap())
    }
}

/// `length` bytes at `offset`, or [`Error::OutOfBounds`] naming `context`
pub(crate) fn range<'a>(
    bytes: &'a [u8],
    offset: usize,
    length: usize,
    context: &'static str,
) -> Result<&'a [u8]> {
    let end = offset.saturating_add(length);
    bytes
        .get(offset..end)
        .ok_or(Error::OutOfBounds { context, offset, end, size: bytes.len() })
}
