use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

/// A bounded view over one VPK entry.
///
/// Preload bytes and the entry's archive range are exposed as one contiguous
/// stream. Reads never continue into the next entry, even when the backing
/// archive contains adjacent files.
#[derive(Debug)]
pub struct EntryReader {
    file: File,
    preload: Vec<u8>,
    data_offset: u64,
    data_length: u64,
    position: u64,
}

impl EntryReader {
    pub(crate) fn new(
        mut file: File,
        preload: Vec<u8>,
        data_offset: u64,
        data_length: u64,
    ) -> io::Result<Self> {
        file.seek(SeekFrom::Start(data_offset))?;
        Ok(Self {
            file,
            preload,
            data_offset,
            data_length,
            position: 0,
        })
    }

    pub fn len(&self) -> u64 {
        self.preload.len() as u64 + self.data_length
    }

    pub fn is_empty(&self) -> bool {
        self.preload.is_empty() && self.data_length == 0
    }

    pub const fn position(&self) -> u64 {
        self.position
    }

    fn preload_length(&self) -> u64 {
        self.preload.len() as u64
    }
}

impl Read for EntryReader {
    fn read(&mut self, mut output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() || self.position >= self.len() {
            return Ok(0);
        }

        let mut written = 0;
        let preload_length = self.preload_length();
        if self.position < preload_length {
            let start = self.position as usize;
            let amount = output.len().min(self.preload.len() - start);
            output[..amount].copy_from_slice(&self.preload[start..start + amount]);
            self.position += amount as u64;
            written += amount;
            output = &mut output[amount..];
        }

        if output.is_empty() || self.position >= self.len() {
            return Ok(written);
        }

        let data_position = self.position - preload_length;
        let remaining = self.data_length - data_position;
        let amount = usize::try_from(remaining.min(output.len() as u64)).unwrap_or(output.len());
        match self.file.read(&mut output[..amount]) {
            Ok(0) if written > 0 => Ok(written),
            Ok(0) => Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "VPK entry data ended before its declared length",
            )),
            Ok(read) => {
                self.position += read as u64;
                Ok(written + read)
            }
            Err(_) if written > 0 => Ok(written),
            Err(error) => Err(error),
        }
    }
}

impl Seek for EntryReader {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let target = match from {
            SeekFrom::Start(position) => i128::from(position),
            SeekFrom::End(offset) => i128::from(self.len()) + i128::from(offset),
            SeekFrom::Current(offset) => i128::from(self.position) + i128::from(offset),
        };

        let target = u64::try_from(target)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "negative seek position"))?;
        let preload_length = self.preload_length();
        let archive_position = target.saturating_sub(preload_length).min(self.data_length);
        let absolute_position = self
            .data_offset
            .checked_add(archive_position)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "seek position overflow"))?;

        self.file.seek(SeekFrom::Start(absolute_position))?;
        self.position = target;
        Ok(target)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Seek, Write};

    use tempfile::NamedTempFile;

    use super::*;

    /// The handle is returned alongside the guard on purpose: dropping the
    /// NamedTempFile deletes the file, so the caller has to keep it alive for
    /// as long as the reader is reading.
    fn temporary_file(contents: &[u8]) -> (NamedTempFile, File) {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(contents).unwrap();
        temp.flush().unwrap();
        let file = File::open(temp.path()).unwrap();
        (temp, file)
    }

    #[test]
    fn reads_preload_and_archive_data_as_one_stream() {
        let (_temp, file) = temporary_file(b"__payload__trailing");
        let mut reader = EntryReader::new(file, b"pre".to_vec(), 2, 9).unwrap();
        let mut output = String::new();

        reader.read_to_string(&mut output).unwrap();
        assert_eq!(output, "prepayload__");
        assert_eq!(reader.len(), 12);
    }

    #[test]
    fn reports_truncated_backing_data() {
        let (_temp, file) = temporary_file(b"abc");
        let mut reader = EntryReader::new(file, Vec::new(), 0, 4).unwrap();
        let mut output = Vec::new();

        let error = reader.read_to_end(&mut output).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
        assert_eq!(output, b"abc");
    }

    #[test]
    fn seek_stays_inside_the_entry_view() {
        let (_temp, file) = temporary_file(b"prefix-data-trailing");
        let mut reader = EntryReader::new(file, b"pre".to_vec(), 7, 4).unwrap();
        let mut output = [0; 3];

        reader.seek(SeekFrom::Start(2)).unwrap();
        reader.read_exact(&mut output).unwrap();
        assert_eq!(&output, b"eda");

        reader.seek(SeekFrom::End(-2)).unwrap();
        let mut tail = String::new();
        reader.read_to_string(&mut tail).unwrap();
        assert_eq!(tail, "ta");
    }
}
