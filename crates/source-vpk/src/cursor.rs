use crate::{Error, Result, Section};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
    section: Section,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(bytes: &'a [u8], section: Section) -> Self {
        Self {
            bytes,
            position: 0,
            section,
        }
    }

    pub(crate) fn position(&self) -> usize {
        self.position
    }

    pub(crate) fn remaining(&self) -> usize {
        self.bytes.len() - self.position
    }

    pub(crate) fn read_u16(&mut self) -> Result<u16> {
        let bytes = self.read_array::<2>()?;
        Ok(u16::from_le_bytes(bytes))
    }

    pub(crate) fn read_u32(&mut self) -> Result<u32> {
        let bytes = self.read_array::<4>()?;
        Ok(u32::from_le_bytes(bytes))
    }

    pub(crate) fn read_c_string(&mut self) -> Result<&'a [u8]> {
        let start = self.position;
        let relative_end = self.bytes[start..]
            .iter()
            .position(|byte| *byte == 0)
            .ok_or(Error::UnterminatedString {
                section: self.section,
                offset: start,
            })?;
        let end = start + relative_end;
        self.position = end + 1;
        Ok(&self.bytes[start..end])
    }

    pub(crate) fn read_bytes(&mut self, length: usize) -> Result<&'a [u8]> {
        let start = self.position;
        let end = start.checked_add(length).ok_or(Error::ArithmeticOverflow {
            context: "cursor range",
        })?;
        if end > self.bytes.len() {
            return Err(self.eof(length));
        }
        self.position = end;
        Ok(&self.bytes[start..end])
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let bytes = self.read_bytes(N)?;
        let mut array = [0; N];
        array.copy_from_slice(bytes);
        Ok(array)
    }

    fn eof(&self, needed: usize) -> Error {
        Error::UnexpectedEof {
            section: self.section,
            offset: self.position,
            needed,
            remaining: self.remaining(),
        }
    }
}
