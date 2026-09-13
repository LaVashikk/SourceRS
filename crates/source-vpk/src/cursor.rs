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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_little_endian_values_and_strings() {
        let bytes = [0x34, 0x12, 0x78, 0x56, 0x34, 0x12, b'a', b'b', 0];
        let mut cursor = Cursor::new(&bytes, Section::Tree);

        assert_eq!(cursor.read_u16().unwrap(), 0x1234);
        assert_eq!(cursor.read_u32().unwrap(), 0x1234_5678);
        assert_eq!(cursor.read_c_string().unwrap(), b"ab");
        assert_eq!(cursor.remaining(), 0);
    }

    #[test]
    fn reports_the_failing_offset() {
        let mut cursor = Cursor::new(&[1, 2, 3], Section::Header);
        let error = cursor.read_u32().unwrap_err();

        assert!(matches!(
            error,
            Error::UnexpectedEof {
                section: Section::Header,
                offset: 0,
                needed: 4,
                remaining: 3,
            }
        ));
    }

    #[test]
    fn rejects_unterminated_strings() {
        let mut cursor = Cursor::new(b"abc", Section::Tree);
        assert!(matches!(
            cursor.read_c_string().unwrap_err(),
            Error::UnterminatedString {
                section: Section::Tree,
                offset: 0,
            }
        ));
    }
}
