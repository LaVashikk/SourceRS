mod common;

use common::{mdl_header, read_i32, studio_fixture, vtx_header, vvd_header, write_i32, write_u16, write_u32};
use source_mdl::{Error, FileKind, Mdl, StudioModel, Vtx, Vvd};

#[test]
fn rejects_signatures_versions_and_declared_length() {
    let fixture = studio_fixture(44);

    let mut mdl = fixture.mdl.clone();
    mdl[0] = b'X';
    assert!(matches!(
        Mdl::from_bytes(&mdl).unwrap_err(),
        Error::InvalidSignature { file: FileKind::Mdl, .. }
    ));
    let mut mdl = fixture.mdl.clone();
    write_i32(&mut mdl, 4, 43);
    assert!(matches!(
        Mdl::from_bytes(&mdl).unwrap_err(),
        Error::UnsupportedVersion { file: FileKind::Mdl, version: 43 }
    ));
    let mut mdl = fixture.mdl.clone();
    write_i32(&mut mdl, mdl_header::LENGTH, 240);
    assert!(matches!(
        Mdl::from_bytes(&mdl).unwrap_err(),
        Error::InvalidField { field: "declared length", .. }
    ));

    let mut vvd = fixture.vvd.clone();
    vvd[0] = b'X';
    assert!(matches!(
        Vvd::from_bytes(&vvd).unwrap_err(),
        Error::InvalidSignature { file: FileKind::Vvd, .. }
    ));
    let mut vtx = fixture.vtx.clone();
    write_i32(&mut vtx, 0, 6);
    assert!(matches!(
        Vtx::from_bytes(&vtx, 44).unwrap_err(),
        Error::UnsupportedVersion { file: FileKind::Vtx, version: 6 }
    ));
}

#[test]
fn rejects_negative_counts_and_bad_ranges() {
    let fixture = studio_fixture(44);

    let mut mdl = fixture.mdl.clone();
    write_i32(&mut mdl, mdl_header::BONES, -1);
    assert!(matches!(Mdl::from_bytes(&mdl).unwrap_err(), Error::InvalidField { file: FileKind::Mdl, .. }));
    let mut mdl = fixture.mdl.clone();
    let outside = mdl.len() as i32 + 1;
    write_i32(&mut mdl, mdl_header::BONES + 4, outside);
    assert!(matches!(Mdl::from_bytes(&mdl).unwrap_err(), Error::OutOfBounds { file: FileKind::Mdl, .. }));

    let mut vvd = fixture.vvd.clone();
    write_i32(&mut vvd, vvd_header::FIXUPS, -1);
    assert!(matches!(Vvd::from_bytes(&vvd).unwrap_err(), Error::InvalidField { file: FileKind::Vvd, .. }));

    let mut vtx = fixture.vtx.clone();
    write_i32(&mut vtx, vtx_header::BODY_PARTS, -1);
    assert!(matches!(
        Vtx::from_bytes(&vtx, 44).unwrap_err(),
        Error::InvalidField { file: FileKind::Vtx, .. }
    ));
}

#[test]
fn rejects_checksum_and_structure_mismatches() {
    let fixture = studio_fixture(44);
    let mut vvd = fixture.vvd.clone();
    write_u32(&mut vvd, vvd_header::CHECKSUM, 0xdead_beef);
    assert!(matches!(
        StudioModel::from_bytes(&fixture.mdl, &vvd, &fixture.vtx).unwrap_err(),
        Error::ChecksumMismatch { file: FileKind::Vvd, .. }
    ));

    let mut vtx = fixture.vtx.clone();
    write_u32(&mut vtx, vtx_header::CHECKSUM, 0xdead_beef);
    assert!(matches!(
        StudioModel::from_bytes(&fixture.mdl, &fixture.vvd, &vtx).unwrap_err(),
        Error::ChecksumMismatch { file: FileKind::Vtx, .. }
    ));

    let mut vtx = fixture.vtx.clone();
    write_i32(&mut vtx, vtx_header::BODY_PARTS, 0);
    assert!(matches!(
        StudioModel::from_bytes(&fixture.mdl, &fixture.vvd, &vtx).unwrap_err(),
        Error::StructureMismatch { context: "body parts", .. }
    ));
}

#[test]
fn rejects_invalid_fixups_and_vtx_indices() {
    let fixture = studio_fixture(44);
    let mut vvd = fixture.vvd.clone();
    write_i32(&mut vvd, vvd_header::LOD_VERTEX_COUNTS + 4, 2);
    assert!(matches!(
        Vvd::from_bytes(&vvd).unwrap_err(),
        Error::StructureMismatch { context: "VVD fixup vertex count", expected: 2, actual: 3 }
    ));

    let mut vtx = fixture.vtx.clone();
    let body = read_i32(&vtx, vtx_header::BODY_PARTS + 4) as usize;
    let model = body + read_i32(&vtx, body + 4) as usize;
    let lod = model + read_i32(&vtx, model + 4) as usize;
    let mesh = lod + read_i32(&vtx, lod + 4) as usize;
    let group = mesh + read_i32(&vtx, mesh + 4) as usize;
    let indices = group + read_i32(&vtx, group + 12) as usize;
    write_u16(&mut vtx, indices, 99);
    assert!(matches!(
        Vtx::from_bytes(&vtx, 44).unwrap_err(),
        Error::InvalidIndex { context: "strip group vertex", .. }
    ));
}

#[test]
fn arbitrary_small_inputs_never_panic() {
    let mut state = 0x6d2b_79f5u32;
    for length in 0..512 {
        let mut bytes = vec![0; length];
        for byte in &mut bytes {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            *byte = state as u8;
        }
        assert!(std::panic::catch_unwind(|| Mdl::from_bytes(&bytes)).is_ok());
        assert!(std::panic::catch_unwind(|| Vvd::from_bytes(&bytes)).is_ok());
        assert!(std::panic::catch_unwind(|| Vtx::from_bytes(&bytes, 49)).is_ok());
    }
}

#[test]
fn every_truncated_fixture_prefix_is_error_not_panic() {
    let fixture = studio_fixture(49);
    for bytes in [&fixture.mdl, &fixture.vvd, &fixture.vtx] {
        for length in 0..bytes.len().min(300) {
            let input = &bytes[..length];
            let result = std::panic::catch_unwind(|| {
                let _ = Mdl::from_bytes(input);
                let _ = Vvd::from_bytes(input);
                let _ = Vtx::from_bytes(input, 49);
            });
            assert!(result.is_ok(), "parser panicked for {length} bytes");
        }
    }
}
