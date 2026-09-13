use indexmap::IndexMap;

use crate::cursor::Cursor;
use crate::entry::{ArchiveIndex, Entry};
use crate::format::ENTRY_TERMINATOR;
use crate::{Error, Result, Section};

pub(crate) fn parse(bytes: &[u8]) -> Result<IndexMap<String, Entry>> {
    if bytes.is_empty() {
        return Ok(IndexMap::new());
    }

    let mut cursor = Cursor::new(bytes, Section::Tree);
    let mut entries = IndexMap::new();

    loop {
        let extension = cursor.read_c_string()?;
        if extension.is_empty() {
            break;
        }

        loop {
            let directory = cursor.read_c_string()?;
            if directory.is_empty() {
                break;
            }

            loop {
                let stem = cursor.read_c_string()?;
                if stem.is_empty() {
                    break;
                }

                let path = build_path(directory, stem, extension);
                let crc32 = cursor.read_u32()?;
                let preload_length = cursor.read_u16()?;
                let archive = ArchiveIndex::from_raw(cursor.read_u16()?);
                let offset = cursor.read_u32()?;
                let data_length = cursor.read_u32()?;
                let terminator_offset = cursor.position();
                let terminator = cursor.read_u16()?;

                if terminator != ENTRY_TERMINATOR {
                    return Err(Error::InvalidEntryTerminator {
                        path,
                        offset: terminator_offset,
                        value: terminator,
                    });
                }

                let preload = cursor.read_bytes(usize::from(preload_length))?.to_vec();
                let entry = Entry::new(crc32, archive, offset, data_length, preload);

                // VPK lookups are case-insensitive. SourcePP keeps the first entry
                // when malformed trees repeat a path after normalization.
                if let indexmap::map::Entry::Vacant(slot) = entries.entry(path) {
                    slot.insert(entry);
                }
            }
        }
    }

    Ok(entries)
}

pub(crate) fn normalize_entry_path(path: &str) -> String {
    let mut normalized = path.replace('\\', "/");
    if normalized.starts_with('/') {
        normalized.remove(0);
    }
    normalized.make_ascii_lowercase();
    normalized
}

fn build_path(directory: &[u8], stem: &[u8], extension: &[u8]) -> String {
    let directory = marker_to_empty(directory);
    let extension = marker_to_empty(extension);

    let mut path = String::new();
    if !directory.is_empty() {
        path.push_str(&decode_component(directory));
        path.push('/');
    }
    path.push_str(&decode_component(stem));
    if !extension.is_empty() {
        path.push('.');
        path.push_str(&decode_component(extension));
    }

    normalize_entry_path(&path)
}

fn marker_to_empty(value: &[u8]) -> &[u8] {
    if value == b" " { b"" } else { value }
}

fn decode_component(value: &[u8]) -> String {
    match std::str::from_utf8(value) {
        Ok(value) => value.to_owned(),
        Err(_) => value.iter().copied().map(char::from).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_c_string(output: &mut Vec<u8>, value: &[u8]) {
        output.extend_from_slice(value);
        output.push(0);
    }

    #[test]
    fn accepts_zero_length_empty_trees() {
        assert!(parse(&[]).unwrap().is_empty());
    }

    #[test]
    fn parses_root_files_without_extensions_and_preload_data() {
        let mut tree = Vec::new();
        push_c_string(&mut tree, b" ");
        push_c_string(&mut tree, b" ");
        push_c_string(&mut tree, b"ReadMe");
        tree.extend_from_slice(&0x1234_5678_u32.to_le_bytes());
        tree.extend_from_slice(&3_u16.to_le_bytes());
        tree.extend_from_slice(&0x7fff_u16.to_le_bytes());
        tree.extend_from_slice(&5_u32.to_le_bytes());
        tree.extend_from_slice(&8_u32.to_le_bytes());
        tree.extend_from_slice(&ENTRY_TERMINATOR.to_le_bytes());
        tree.extend_from_slice(b"abc");
        push_c_string(&mut tree, b"");
        push_c_string(&mut tree, b"");
        push_c_string(&mut tree, b"");

        let entries = parse(&tree).unwrap();
        let entry = entries.get("readme").unwrap();
        assert_eq!(entry.crc32(), 0x1234_5678);
        assert_eq!(entry.preload(), b"abc");
        assert_eq!(entry.data_length(), 8);
        assert_eq!(entry.offset(), 5);
    }

    #[test]
    fn normalizes_lookup_paths_like_sourcepp() {
        assert_eq!(
            normalize_entry_path("/Materials\\Brick.VMT"),
            "materials/brick.vmt"
        );
    }

    #[test]
    fn rejects_bad_entry_terminators() {
        let mut tree = Vec::new();
        push_c_string(&mut tree, b"txt");
        push_c_string(&mut tree, b"scripts");
        push_c_string(&mut tree, b"test");
        tree.extend_from_slice(&[0; 16]);
        tree.extend_from_slice(&0_u16.to_le_bytes());

        assert!(matches!(
            parse(&tree).unwrap_err(),
            Error::InvalidEntryTerminator { .. }
        ));
    }
}
