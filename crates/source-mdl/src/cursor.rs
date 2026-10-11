use std::iter::StepBy;
use std::ops::Range;

use crate::{Error, FileKind, Mat3x4, Result, Vec3, Vec4};

/// Whole-file view that resolves the offsets studio files link their records with
///
/// Almost every offset in MDL and VTX is relative to the start of the record that holds it.
/// Offsets stored in the file header are relative to byte 0.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    file: FileKind,
}

impl<'a> Reader<'a> {
    pub(crate) const fn new(bytes: &'a [u8], file: FileKind) -> Self {
        Self { bytes, file }
    }

    pub(crate) const fn len(self) -> usize {
        self.bytes.len()
    }

    pub(crate) fn at(self, offset: usize, context: &'static str) -> Result<Cursor<'a>> {
        if offset > self.bytes.len() {
            return Err(self.out_of_bounds(context, offset, offset));
        }
        Ok(Cursor { reader: self, position: offset, context })
    }

    pub(crate) fn offset(self, base: usize, relative: i32, context: &'static str) -> Result<usize> {
        match base.checked_add_signed(relative as isize) {
            Some(offset) if offset <= self.bytes.len() => Ok(offset),
            _ => Err(self.out_of_bounds(context, base, base.saturating_add_signed(relative as isize))),
        }
    }

    /// Start offsets of `count` records of `size` bytes, `relative` bytes from `base`.
    /// For records that link further records, flat data goes through [`Self::records`]
    pub(crate) fn array(
        self,
        base: usize,
        relative: i32,
        count: i32,
        size: usize,
        context: &'static str,
    ) -> Result<StepBy<Range<usize>>> {
        // TODO: hmmmm nothing stops a file from pointing a million records at the same nested array, but how much is real?
        let count = self.nonnegative(count, context)?;
        Ok(self
            .span(base, relative, count, size, context)?
            .step_by(size))
    }

    /// `count` records of `N` bytes, `relative` bytes from `base`.
    /// The whole span is checked once, so records are decoded with the `*_at` helpers below instead of a Cursor
    pub(crate) fn records<const N: usize>(
        self,
        base: usize,
        relative: i32,
        count: usize,
        context: &'static str,
    ) -> Result<impl ExactSizeIterator<Item = &'a [u8; N]>> {
        let span = self.span(base, relative, count, N, context)?;
        // todo: cool comm here later
        Ok(self.bytes[span]
            .chunks_exact(N)
            .map(|record| record.try_into().unwrap()))
    }

    fn span(
        self,
        base: usize,
        relative: i32,
        count: usize,
        size: usize,
        context: &'static str,
    ) -> Result<Range<usize>> {
        // studiomdl leaves stale offsets in empty arrays, so those are not checked at all
        if count == 0 {
            return Ok(0..0);
        }

        let start = self.offset(base, relative, context)?;

        match count
            .checked_mul(size)
            .and_then(|length| start.checked_add(length))
        {
            Some(end) if end <= self.bytes.len() => Ok(start..end),
            end => Err(self.out_of_bounds(context, start, end.unwrap_or(usize::MAX))),
        }
    }

    /// NUL-terminated string `relative` bytes from `base`; a zero offset means no string
    pub(crate) fn string(self, base: usize, relative: i32, context: &'static str) -> Result<String> {
        if relative == 0 {
            return Ok(String::new());
        }

        let start = self.offset(base, relative, context)?;
        let tail = &self.bytes[start..];
        let length = tail
            .iter()
            .position(|&byte| byte == 0)
            .ok_or_else(|| self.out_of_bounds(context, start, self.bytes.len() + 1))?;

        Ok(String::from_utf8_lossy(&tail[..length]).into_owned())
    }

    pub(crate) fn fixed_string(self, offset: usize, length: usize, context: &'static str) -> Result<String> {
        let end = offset.saturating_add(length);
        let bytes = self
            .bytes
            .get(offset..end)
            .ok_or_else(|| self.out_of_bounds(context, offset, end))?;

        let used = bytes
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(length);

        Ok(String::from_utf8_lossy(&bytes[..used]).into_owned())
    }

    pub(crate) fn nonnegative(self, value: i32, field: &'static str) -> Result<usize> {
        usize::try_from(value).map_err(|_| self.invalid(field, value, "cannot be negative"))
    }

    /// Checks an element range such as a strip's slice of its group's indices
    pub(crate) fn subrange(
        self,
        offset: usize,
        count: usize,
        size: usize,
        context: &'static str,
    ) -> Result<()> {
        match offset.checked_add(count) {
            Some(end) if end <= size => Ok(()),
            end => Err(Error::OutOfBounds {
                file: self.file,
                context,
                offset,
                end: end.unwrap_or(usize::MAX),
                size,
            }),
        }
    }

    pub(crate) fn invalid(self, field: &'static str, value: impl Into<i64>, reason: &'static str) -> Error {
        Error::InvalidField { file: self.file, field, value: value.into(), reason }
    }

    fn out_of_bounds(self, context: &'static str, offset: usize, end: usize) -> Error {
        Error::OutOfBounds { file: self.file, context, offset, end, size: self.bytes.len() }
    }
}

/// `value` as an index below `count`
pub(crate) fn index(value: impl Into<i64>, count: usize, context: &'static str) -> Result<usize> {
    let value = value.into();
    usize::try_from(value)
        .ok()
        .filter(|&index| index < count)
        .ok_or(Error::InvalidIndex { context, index: value, count })
}

// Field reads for records from Reader::records, which already checked the whole span
// Offsets are constants inside the record size, so the indexing never fails
// Keep the #[inline]: without it these stay calls across codegen units and every bounds check comes back, perf caught vec3_at as its own symbol
#[inline]
pub(crate) fn u16_at(record: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([record[at], record[at + 1]])
}

#[inline]
pub(crate) fn i16_at(record: &[u8], at: usize) -> i16 {
    i16::from_le_bytes([record[at], record[at + 1]])
}

#[inline]
pub(crate) fn f32_at(record: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([record[at], record[at + 1], record[at + 2], record[at + 3]])
}

#[inline]
pub(crate) fn vec3_at(record: &[u8], at: usize) -> Vec3 {
    [f32_at(record, at), f32_at(record, at + 4), f32_at(record, at + 8)]
}

/// Sequential little-endian reads inside one record
#[derive(Debug, Clone, Copy)]
pub(crate) struct Cursor<'a> {
    reader: Reader<'a>,
    position: usize,
    context: &'static str,
}

impl<'a> Cursor<'a> {
    pub(crate) fn skip(&mut self, length: usize) -> Result<()> {
        self.take(length).map(drop)
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    pub(crate) fn read_u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    pub(crate) fn read_i16(&mut self) -> Result<i16> {
        Ok(i16::from_le_bytes(self.array()?))
    }

    pub(crate) fn read_u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    pub(crate) fn read_i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.array()?))
    }

    pub(crate) fn read_f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.array()?))
    }

    pub(crate) fn read_vec3(&mut self) -> Result<Vec3> {
        Ok([self.read_f32()?, self.read_f32()?, self.read_f32()?])
    }

    pub(crate) fn read_vec4(&mut self) -> Result<Vec4> {
        Ok([self.read_f32()?, self.read_f32()?, self.read_f32()?, self.read_f32()?])
    }

    pub(crate) fn read_mat3x4(&mut self) -> Result<Mat3x4> {
        Ok([self.read_vec4()?, self.read_vec4()?, self.read_vec4()?])
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self.position.saturating_add(length);
        let bytes = self
            .reader
            .bytes
            .get(self.position..end)
            .ok_or_else(|| {
                self.reader
                    .out_of_bounds(self.context, self.position, end)
            })?;
        self.position = end;
        Ok(bytes)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut output = [0; N];
        output.copy_from_slice(self.take(N)?);
        Ok(output)
    }
}
