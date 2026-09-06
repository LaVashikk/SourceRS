use crate::{Error, Result, Section};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
    section: Section,
}

impl<'a> Cursor<'a> {
    pub(crate) const fn new(bytes: &'a [u8], section: Section) -> Self {
        Self {
            bytes,
            position: 0,
            section,
        }
    }

    pub(crate) const fn position(&self) -> usize {
        self.position
    }

    pub(crate) const fn remaining(&self) -> usize {
        self.bytes.len() - self.position
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8> {
        Ok(self.read_array::<1>()?[0])
    }

    pub(crate) fn read_u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.read_array()?))
    }

    pub(crate) fn read_u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.read_array()?))
    }

    pub(crate) fn read_i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.read_array()?))
    }

    pub(crate) fn read_f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.read_array()?))
    }

    pub(crate) fn skip(&mut self, length: usize) -> Result<()> {
        self.read_bytes(length).map(|_| ())
    }

    pub(crate) fn read_bytes(&mut self, length: usize) -> Result<&'a [u8]> {
        let start = self.position;
        let end = start.checked_add(length).ok_or(Error::ArithmeticOverflow {
            context: "cursor range",
        })?;
        let bytes = self.bytes.get(start..end).ok_or_else(|| self.eof(length))?;
        self.position = end;
        Ok(bytes)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut output = [0; N];
        output.copy_from_slice(self.read_bytes(N)?);
        Ok(output)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_values_and_tracks_the_failing_offset() {
        let bytes = [7, 0x34, 0x12, 0, 0, 0x80, 0x3f];
        let mut cursor = Cursor::new(&bytes, Section::Header);

        assert_eq!(cursor.read_u8().unwrap(), 7);
        assert_eq!(cursor.read_u16().unwrap(), 0x1234);
        assert_eq!(cursor.read_f32().unwrap(), 1.0);
        assert!(matches!(
            cursor.read_u8().unwrap_err(),
            Error::UnexpectedEof {
                section: Section::Header,
                offset: 7,
                needed: 1,
                remaining: 0,
            }
        ));
    }
}
