use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{Error, PackFile};

/// One location inside a search path
pub enum Mount<P> {
    Dir(PathBuf),
    Pack(Arc<P>),
}

impl<P> Clone for Mount<P> {
    fn clone(&self) -> Self {
        match self {
            Self::Dir(path) => Self::Dir(path.clone()),
            Self::Pack(pack) => Self::Pack(Arc::clone(pack)),
        }
    }
}

// Compares packs by Arc pointer identity
impl<P> PartialEq for Mount<P> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Dir(a), Self::Dir(b)) => a == b,
            (Self::Pack(a), Self::Pack(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl<P> std::fmt::Debug for Mount<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dir(path) => f.debug_tuple("Dir").field(path).finish(),
            Self::Pack(_) => f.write_str("Pack(..)"),
        }
    }
}

/// Where a file was found
pub enum FileLocation<P: PackFile = crate::DefaultPack> {
    Loose(PathBuf),
    /// `entry` is the normalized path inside `pack`
    Packed { pack: Arc<P>, entry: String },
}

impl<P: PackFile> FileLocation<P> {
    pub fn open(&self) -> Result<FileReader<'_, P>, Error> {
        match self {
            Self::Loose(path) => open_loose(path),
            Self::Packed { pack, entry } => open_packed(pack, entry),
        }
    }

    pub fn read(&self) -> Result<Vec<u8>, Error> {
        let path = match self {
            Self::Loose(path) => path.to_string_lossy(),
            Self::Packed { entry, .. } => entry.into(),
        };
        read_all(self.open()?, &path)
    }
}

impl<P: PackFile> Clone for FileLocation<P> {
    fn clone(&self) -> Self {
        match self {
            Self::Loose(path) => Self::Loose(path.clone()),
            Self::Packed { pack, entry } => Self::Packed {
                pack: Arc::clone(pack),
                entry: entry.clone(),
            },
        }
    }
}

impl<P: PackFile> std::fmt::Debug for FileLocation<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Loose(path) => f.debug_tuple("Loose").field(path).finish(),
            Self::Packed { entry, .. } => f.debug_struct("Packed").field("entry", entry).finish_non_exhaustive(),
        }
    }
}

/// Streaming file handle returned by the virtual filesystem
pub enum FileReader<'a, P: PackFile + 'a> {
    Loose(std::fs::File),
    Packed(P::Reader<'a>),
}

impl<'a, P: PackFile + 'a> Read for FileReader<'a, P> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Loose(file) => file.read(buffer),
            Self::Packed(file) => file.read(buffer),
        }
    }
}

impl<'a, P: PackFile + 'a> Seek for FileReader<'a, P> {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        match self {
            Self::Loose(file) => file.seek(position),
            Self::Packed(file) => file.seek(position),
        }
    }
}

pub(crate) fn open_loose<'a, P: PackFile>(path: &Path) -> Result<FileReader<'a, P>, Error> {
    std::fs::File::open(path)
        .map(FileReader::Loose)
        .map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })
}

pub(crate) fn open_packed<'a, P: PackFile>(pack: &'a P, entry: &str) -> Result<FileReader<'a, P>, Error> {
    pack.open_entry(entry)
        .map(FileReader::Packed)
        .map_err(|error| Error::pack_entry(entry.to_owned(), error))
}

pub(crate) fn read_all<P: PackFile>(mut reader: FileReader<'_, P>, path: &str) -> Result<Vec<u8>, Error> {
    let mut data = Vec::new();
    // todo: optimize it later
    reader.read_to_end(&mut data).map_err(|source| Error::Io {
        path: PathBuf::from(path),
        source,
    })?;
    Ok(data)
}
