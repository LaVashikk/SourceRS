use std::fs;
use std::io::{Read, Seek, SeekFrom};

use md5::{Digest, Md5};
use source_vpk::{ArchiveIndex, Error, Version, Vpk};
use tempfile::tempdir;

const SIGNATURE: u32 = 0x55aa_1234;
const ENTRY_TERMINATOR: u16 = 0xffff;
const DIRECTORY_ARCHIVE_INDEX: u16 = 0x7fff;

#[test]
fn reads_v1_external_entries_and_resolves_numbered_paths() {
    let directory = tempdir().unwrap();
    let directory_path = directory.path().join("pak01_dir.vpk");
    let archive_path = directory.path().join("pak01_002.vpk");
    let preload = b"pre";
    let archive_data = b"archive";
    let mut complete_data = preload.to_vec();
    complete_data.extend_from_slice(archive_data);

    let tree = one_entry_tree(EntryFixture {
        extension: b"txt",
        directory: b"scripts",
        stem: b"example",
        crc32: crc32fast::hash(&complete_data),
        archive_index: 2,
        offset: 2,
        data: archive_data,
        preload,
    });
    fs::write(&directory_path, v1_file(&tree, b"")).unwrap();
    fs::write(
        &archive_path,
        [b"__".as_slice(), archive_data, b"tail"].concat(),
    )
    .unwrap();

    let vpk = Vpk::open(&archive_path).unwrap();
    assert_eq!(vpk.path(), directory_path);
    assert_eq!(vpk.version(), Version::V1);
    assert!(vpk.contains("/SCRIPTS\\EXAMPLE.TXT"));

    let entry = vpk.entry("scripts/example.txt").unwrap();
    assert_eq!(entry.archive(), ArchiveIndex::External(2));
    assert_eq!(entry.preload(), preload);
    assert_eq!(entry.data_length(), archive_data.len() as u32);
    assert_eq!(vpk.read("scripts/example.txt").unwrap(), complete_data);
    vpk.verify_entry("scripts/example.txt").unwrap();

    let mut reader = vpk.open_entry("scripts/example.txt").unwrap();
    reader.seek(SeekFrom::Start(2)).unwrap();
    let mut tail = String::new();
    reader.read_to_string(&mut tail).unwrap();
    assert_eq!(tail, "earchive");
}

#[test]
fn reads_v2_embedded_entries_and_verifies_footer_hashes() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("pak01_dir.vpk");
    let payload = b"embedded file data";
    let tree = one_entry_tree(EntryFixture {
        extension: b"vmt",
        directory: b"materials",
        stem: b"brick",
        crc32: crc32fast::hash(payload),
        archive_index: DIRECTORY_ARCHIVE_INDEX,
        offset: 0,
        data: payload,
        preload: b"",
    });
    fs::write(&path, v2_file(&tree, payload)).unwrap();

    let vpk = Vpk::open(&path).unwrap();
    assert_eq!(vpk.version(), Version::V2);
    assert_eq!(vpk.len(), 1);
    assert_eq!(vpk.read("materials/brick.vmt").unwrap(), payload);
    vpk.verify_entry("materials/brick.vmt").unwrap();

    let verification = vpk.verify_archive().unwrap().unwrap();
    assert!(verification.is_valid());
    assert!(vpk.archive_md5_entries().is_empty());
}

#[test]
fn exposes_v2_archive_md5_records() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("pak01_dir.vpk");
    let tree = vec![0];
    let mut archive_md5 = Vec::new();
    archive_md5.extend_from_slice(&7_u32.to_le_bytes());
    archive_md5.extend_from_slice(&1024_u32.to_le_bytes());
    archive_md5.extend_from_slice(&4096_u32.to_le_bytes());
    archive_md5.extend_from_slice(&[0xab; 16]);
    fs::write(&path, v2_file_with_archive_md5(&tree, b"", &archive_md5)).unwrap();

    let vpk = Vpk::open(path).unwrap();
    let record = &vpk.archive_md5_entries()[0];
    assert_eq!(record.archive_index, 7);
    assert_eq!(record.offset, 1024);
    assert_eq!(record.length, 4096);
    assert_eq!(record.checksum, [0xab; 16]);
    assert!(vpk.verify_archive().unwrap().unwrap().is_valid());
}

#[test]
fn bounds_embedded_entries_to_the_v2_file_data_section() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("bad_dir.vpk");
    let tree = one_entry_tree(EntryFixture {
        extension: b"bin",
        directory: b" ",
        stem: b"bad",
        crc32: 0,
        archive_index: DIRECTORY_ARCHIVE_INDEX,
        offset: 1,
        data: b"too long",
        preload: b"",
    });
    fs::write(&path, v2_file(&tree, b"x")).unwrap();

    assert!(matches!(
        Vpk::open(path).unwrap_err(),
        Error::EntryOutOfBounds { .. }
    ));
}

#[test]
fn reports_entry_crc_mismatches() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("pak01_dir.vpk");
    let payload = b"payload";
    let tree = one_entry_tree(EntryFixture {
        extension: b"dat",
        directory: b" ",
        stem: b"file",
        crc32: 0xdead_beef,
        archive_index: DIRECTORY_ARCHIVE_INDEX,
        offset: 0,
        data: payload,
        preload: b"",
    });
    fs::write(&path, v1_file(&tree, payload)).unwrap();
    let vpk = Vpk::open(path).unwrap();

    assert!(matches!(
        vpk.verify_entry("file.dat").unwrap_err(),
        Error::EntryChecksumMismatch { .. }
    ));
}

#[test]
fn reports_the_truncated_v2_section() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("truncated_dir.vpk");
    let mut header = Vec::new();
    header.extend_from_slice(&SIGNATURE.to_le_bytes());
    header.extend_from_slice(&2_u32.to_le_bytes());
    header.extend_from_slice(&10_u32.to_le_bytes());
    header.extend_from_slice(&[0; 16]);
    fs::write(&path, header).unwrap();

    assert!(matches!(
        Vpk::open(path).unwrap_err(),
        Error::SectionOutOfBounds {
            section: source_vpk::Section::Tree,
            ..
        }
    ));
}

#[test]
fn rejects_unreasonably_large_index_sections_before_allocating() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("huge_dir.vpk");
    let tree_size = 512_u32 * 1024 * 1024 + 1;
    let mut header = Vec::new();
    header.extend_from_slice(&SIGNATURE.to_le_bytes());
    header.extend_from_slice(&1_u32.to_le_bytes());
    header.extend_from_slice(&tree_size.to_le_bytes());
    fs::write(&path, header).unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(12 + u64::from(tree_size))
        .unwrap();

    assert!(matches!(
        Vpk::open(path).unwrap_err(),
        Error::InvalidSectionSize { .. }
    ));
}

#[test]
fn arbitrary_small_inputs_do_not_panic() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("fuzz_dir.vpk");
    let mut state = 0x6d2b_79f5_u32;

    for length in 0..256 {
        let mut bytes = vec![0; length];
        for byte in &mut bytes {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            *byte = state as u8;
        }
        fs::write(&path, &bytes).unwrap();
        let result = std::panic::catch_unwind(|| Vpk::open(&path));
        assert!(result.is_ok(), "parser panicked for {length} bytes");
    }
}

struct EntryFixture<'a> {
    extension: &'a [u8],
    directory: &'a [u8],
    stem: &'a [u8],
    crc32: u32,
    archive_index: u16,
    offset: u32,
    data: &'a [u8],
    preload: &'a [u8],
}

fn one_entry_tree(entry: EntryFixture<'_>) -> Vec<u8> {
    let mut tree = Vec::new();
    push_c_string(&mut tree, entry.extension);
    push_c_string(&mut tree, entry.directory);
    push_c_string(&mut tree, entry.stem);
    tree.extend_from_slice(&entry.crc32.to_le_bytes());
    tree.extend_from_slice(&(entry.preload.len() as u16).to_le_bytes());
    tree.extend_from_slice(&entry.archive_index.to_le_bytes());
    tree.extend_from_slice(&entry.offset.to_le_bytes());
    tree.extend_from_slice(&(entry.data.len() as u32).to_le_bytes());
    tree.extend_from_slice(&ENTRY_TERMINATOR.to_le_bytes());
    tree.extend_from_slice(entry.preload);
    push_c_string(&mut tree, b"");
    push_c_string(&mut tree, b"");
    push_c_string(&mut tree, b"");
    tree
}

fn v1_file(tree: &[u8], file_data: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    output.extend_from_slice(&SIGNATURE.to_le_bytes());
    output.extend_from_slice(&1_u32.to_le_bytes());
    output.extend_from_slice(&(tree.len() as u32).to_le_bytes());
    output.extend_from_slice(tree);
    output.extend_from_slice(file_data);
    output
}

fn v2_file(tree: &[u8], file_data: &[u8]) -> Vec<u8> {
    v2_file_with_archive_md5(tree, file_data, &[])
}

fn v2_file_with_archive_md5(tree: &[u8], file_data: &[u8], archive_md5: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    output.extend_from_slice(&SIGNATURE.to_le_bytes());
    output.extend_from_slice(&2_u32.to_le_bytes());
    output.extend_from_slice(&(tree.len() as u32).to_le_bytes());
    output.extend_from_slice(&(file_data.len() as u32).to_le_bytes());
    output.extend_from_slice(&(archive_md5.len() as u32).to_le_bytes());
    output.extend_from_slice(&48_u32.to_le_bytes());
    output.extend_from_slice(&0_u32.to_le_bytes());
    output.extend_from_slice(tree);
    output.extend_from_slice(file_data);
    output.extend_from_slice(archive_md5);
    output.extend_from_slice(&md5(tree));
    output.extend_from_slice(&md5(archive_md5));
    let whole_file = md5(&output);
    output.extend_from_slice(&whole_file);
    output
}

fn push_c_string(output: &mut Vec<u8>, value: &[u8]) {
    output.extend_from_slice(value);
    output.push(0);
}

fn md5(bytes: &[u8]) -> [u8; 16] {
    Md5::digest(bytes).into()
}
