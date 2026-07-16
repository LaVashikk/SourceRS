use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crc32fast::Hasher as Crc32;
use indexmap::IndexMap;
use md5::{Digest, Md5};

use crate::entry::{ArchiveIndex, Entry};
use crate::format::{
    self, ArchiveMd5Entry, FooterChecksums, Layout, Version, parse_archive_md5_entries,
    parse_footer_checksums,
};
use crate::reader::EntryReader;
use crate::tree::{self, normalize_entry_path};
use crate::{Error, Result, Section};

const HEADER_READ_SIZE: u64 = 28;
const COPY_BUFFER_SIZE: usize = 64 * 1024;
const MAX_INDEX_SECTION_SIZE: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ArchiveVerification {
    pub tree: bool,
    pub archive_md5_section: bool,
    pub whole_file: bool,
}

impl ArchiveVerification {
    pub const fn is_valid(self) -> bool {
        self.tree && self.archive_md5_section && self.whole_file
    }
}

#[derive(Debug)]
pub struct Vpk {
    path: PathBuf,
    version: Version,
    entries: IndexMap<String, Entry>,
    layout: Layout,
    archive_md5_entries: Vec<ArchiveMd5Entry>,
    footer_checksums: Option<FooterChecksums>,
}

impl Vpk {
    /// Opens a VPK directory file and parses its entry index.
    ///
    /// Passing a numbered archive such as `pak01_003.vpk` resolves to a sibling
    /// `pak01_dir.vpk` when it exists, matching SourcePP's convenience behavior.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = resolve_directory_path(path.as_ref())?;
        let mut file = File::open(&path).map_err(|error| Error::io(&path, error))?;
        let file_size = file
            .metadata()
            .map_err(|error| Error::io(&path, error))?
            .len();

        let header_bytes = read_range(
            &mut file,
            &path,
            0,
            file_size.min(HEADER_READ_SIZE),
            "VPK header",
        )?;
        let header = format::parse_header(&header_bytes)?;
        let layout = header.layout(file_size)?;
        ensure_index_section_size(Section::Tree, layout.tree_size)?;
        ensure_index_section_size(Section::ArchiveMd5, layout.archive_md5_size)?;
        let tree_bytes = read_range(
            &mut file,
            &path,
            layout.tree_offset,
            layout.tree_size,
            "VPK directory tree",
        )?;
        let entries = tree::parse(&tree_bytes)?;

        validate_embedded_entries(&path, file_size, layout, &entries)?;

        let archive_md5_entries = match layout.archive_md5_offset {
            Some(offset) if layout.archive_md5_size > 0 => {
                let bytes = read_range(
                    &mut file,
                    &path,
                    offset,
                    layout.archive_md5_size,
                    "VPK archive MD5 section",
                )?;
                parse_archive_md5_entries(&bytes)?
            }
            _ => Vec::new(),
        };

        let footer_checksums = match layout.other_md5_offset {
            Some(offset) if layout.other_md5_size == 48 => {
                let bytes = read_range(
                    &mut file,
                    &path,
                    offset,
                    layout.other_md5_size,
                    "VPK other MD5 section",
                )?;
                Some(parse_footer_checksums(&bytes)?)
            }
            _ => None,
        };

        Ok(Self {
            path,
            version: header.version,
            entries,
            layout,
            archive_md5_entries,
            footer_checksums,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> impl ExactSizeIterator<Item = (&str, &Entry)> {
        self.entries
            .iter()
            .map(|(path, entry)| (path.as_str(), entry))
    }

    pub fn entry(&self, path: &str) -> Option<&Entry> {
        self.entries.get(&normalize_entry_path(path))
    }

    pub fn contains(&self, path: &str) -> bool {
        self.entry(path).is_some()
    }

    pub fn archive_md5_entries(&self) -> &[ArchiveMd5Entry] {
        &self.archive_md5_entries
    }

    pub fn open_entry(&self, path: &str) -> Result<EntryReader> {
        let normalized = normalize_entry_path(path);
        let entry = self
            .entries
            .get(&normalized)
            .ok_or_else(|| Error::EntryNotFound(normalized.clone()))?;
        let (archive_path, data_offset) = self.entry_location(entry)?;
        let file = File::open(&archive_path).map_err(|error| Error::io(&archive_path, error))?;
        let file_size = file
            .metadata()
            .map_err(|error| Error::io(&archive_path, error))?
            .len();
        let end = data_offset
            .checked_add(u64::from(entry.data_length()))
            .ok_or(Error::ArithmeticOverflow {
                context: "entry data end",
            })?;

        if end > file_size {
            return Err(Error::EntryOutOfBounds {
                path: normalized,
                archive: archive_path,
                offset: data_offset,
                end,
                file_size,
            });
        }

        EntryReader::new(
            file,
            entry.preload().to_vec(),
            data_offset,
            u64::from(entry.data_length()),
        )
        .map_err(|error| Error::io(&archive_path, error))
    }

    pub fn read(&self, path: &str) -> Result<Vec<u8>> {
        let mut reader = self.open_entry(path)?;
        let length = reader.len();
        let capacity = usize::try_from(length).map_err(|_| Error::Allocation {
            context: "VPK entry",
            size: length,
        })?;
        let mut output = Vec::new();
        output
            .try_reserve_exact(capacity)
            .map_err(|_| Error::Allocation {
                context: "VPK entry",
                size: length,
            })?;
        reader
            .read_to_end(&mut output)
            .map_err(|error| Error::io(self.path_for_entry(path), error))?;
        Ok(output)
    }

    pub fn verify_entry(&self, path: &str) -> Result<()> {
        let normalized = normalize_entry_path(path);
        let entry = self
            .entries
            .get(&normalized)
            .ok_or_else(|| Error::EntryNotFound(normalized.clone()))?;
        let mut reader = self.open_entry(&normalized)?;
        let mut hasher = Crc32::new();
        let mut buffer = [0; COPY_BUFFER_SIZE];

        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(|error| Error::io(self.path_for_entry(&normalized), error))?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }

        let actual = hasher.finalize();
        if actual != entry.crc32() {
            return Err(Error::EntryChecksumMismatch {
                path: normalized,
                expected: entry.crc32(),
                actual,
            });
        }
        Ok(())
    }

    /// Verifies the three MD5 values stored in a VPK v2 directory footer.
    ///
    /// `None` means the archive is v1 or has no standard 48-byte checksum
    /// section. External archive chunk hashes are exposed separately through
    /// [`Vpk::archive_md5_entries`].
    pub fn verify_archive(&self) -> Result<Option<ArchiveVerification>> {
        let Some(expected) = self.footer_checksums else {
            return Ok(None);
        };

        let mut file = File::open(&self.path).map_err(|error| Error::io(&self.path, error))?;
        let tree = md5_range(
            &mut file,
            &self.path,
            Section::Tree,
            self.layout.tree_offset,
            self.layout.tree_size,
        )?;
        let archive_md5 = md5_range(
            &mut file,
            &self.path,
            Section::ArchiveMd5,
            self.layout.archive_md5_offset.unwrap_or(0),
            self.layout.archive_md5_size,
        )?;
        let whole_file_length = self
            .layout
            .other_md5_offset
            .and_then(|offset| offset.checked_add(32))
            .ok_or(Error::ArithmeticOverflow {
                context: "VPK whole-file checksum range",
            })?;
        let whole_file = md5_range(
            &mut file,
            &self.path,
            Section::OtherMd5,
            0,
            whole_file_length,
        )?;

        Ok(Some(ArchiveVerification {
            tree: tree == expected.tree,
            archive_md5_section: archive_md5 == expected.archive_md5_section,
            whole_file: whole_file == expected.whole_file,
        }))
    }

    fn entry_location(&self, entry: &Entry) -> Result<(PathBuf, u64)> {
        match entry.archive() {
            ArchiveIndex::Directory => {
                let offset = self
                    .layout
                    .file_data_offset
                    .checked_add(u64::from(entry.offset()))
                    .ok_or(Error::ArithmeticOverflow {
                        context: "directory entry offset",
                    })?;
                Ok((self.path.clone(), offset))
            }
            ArchiveIndex::External(index) => Ok((
                numbered_archive_path(&self.path, index)?,
                u64::from(entry.offset()),
            )),
        }
    }

    fn path_for_entry(&self, path: &str) -> PathBuf {
        self.entry(path)
            .and_then(|entry| self.entry_location(entry).ok())
            .map(|(path, _)| path)
            .unwrap_or_else(|| self.path.clone())
    }
}

fn validate_embedded_entries(
    path: &Path,
    file_size: u64,
    layout: Layout,
    entries: &IndexMap<String, Entry>,
) -> Result<()> {
    for (entry_path, entry) in entries {
        if entry.archive() != ArchiveIndex::Directory {
            continue;
        }

        let offset = layout
            .file_data_offset
            .checked_add(u64::from(entry.offset()))
            .ok_or(Error::ArithmeticOverflow {
                context: "embedded entry offset",
            })?;
        let end = offset.checked_add(u64::from(entry.data_length())).ok_or(
            Error::ArithmeticOverflow {
                context: "embedded entry end",
            },
        )?;
        let section_end = match layout.file_data_size {
            Some(size) => {
                layout
                    .file_data_offset
                    .checked_add(size)
                    .ok_or(Error::ArithmeticOverflow {
                        context: "VPK file data section end",
                    })?
            }
            None => file_size,
        };

        if end > section_end {
            return Err(Error::EntryOutOfBounds {
                path: entry_path.clone(),
                archive: path.to_path_buf(),
                offset,
                end,
                file_size: section_end,
            });
        }
    }
    Ok(())
}

fn resolve_directory_path(path: &Path) -> Result<PathBuf> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::InvalidArchivePath(path.to_path_buf()))?;
    let lower = file_name.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let numbered = bytes.len() >= 8
        && bytes.ends_with(b".vpk")
        && bytes[bytes.len() - 8] == b'_'
        && bytes[bytes.len() - 7..bytes.len() - 4]
            .iter()
            .all(u8::is_ascii_digit);

    if !numbered {
        return Ok(path.to_path_buf());
    }

    let directory_name = format!("{}_dir.vpk", &file_name[..file_name.len() - 8]);
    let directory_path = path.with_file_name(directory_name);
    Ok(if directory_path.exists() {
        directory_path
    } else {
        path.to_path_buf()
    })
}

fn numbered_archive_path(directory_path: &Path, index: u16) -> Result<PathBuf> {
    let stem = directory_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| Error::InvalidArchivePath(directory_path.to_path_buf()))?;
    let base = if stem.len() >= 4 && stem[stem.len() - 4..].eq_ignore_ascii_case("_dir") {
        &stem[..stem.len() - 4]
    } else {
        stem
    };
    Ok(directory_path.with_file_name(format!("{base}_{index:03}.vpk")))
}

fn ensure_index_section_size(section: Section, size: u64) -> Result<()> {
    if size > MAX_INDEX_SECTION_SIZE {
        return Err(Error::InvalidSectionSize {
            section,
            size,
            reason: "the index section exceeds the 512 MiB safety limit",
        });
    }
    Ok(())
}

fn read_range(
    file: &mut File,
    path: &Path,
    offset: u64,
    length: u64,
    context: &'static str,
) -> Result<Vec<u8>> {
    let length = usize::try_from(length).map_err(|_| Error::Allocation {
        context,
        size: length,
    })?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(length)
        .map_err(|_| Error::Allocation {
            context,
            size: length as u64,
        })?;
    output.resize(length, 0);
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| Error::io(path, error))?;
    file.read_exact(&mut output)
        .map_err(|error| Error::io(path, error))?;
    Ok(output)
}

fn md5_range(
    file: &mut File,
    path: &Path,
    section: Section,
    offset: u64,
    length: u64,
) -> Result<[u8; 16]> {
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| Error::io(path, error))?;
    let mut remaining = length;
    let mut buffer = [0; COPY_BUFFER_SIZE];
    let mut hasher = Md5::new();

    while remaining > 0 {
        let wanted = usize::try_from(remaining.min(buffer.len() as u64)).unwrap_or(buffer.len());
        let read = file
            .read(&mut buffer[..wanted])
            .map_err(|error| Error::io(path, error))?;
        if read == 0 {
            return Err(Error::UnexpectedEof {
                section,
                offset: usize::try_from(length - remaining).unwrap_or(usize::MAX),
                needed: wanted,
                remaining: 0,
            });
        }
        hasher.update(&buffer[..read]);
        remaining -= read as u64;
    }

    Ok(hasher.finalize().into())
}
