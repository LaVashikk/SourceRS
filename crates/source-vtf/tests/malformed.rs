mod common;

use common::{Fixture, header, write_i32, write_u16, write_u32};
use source_vtf::{Error, ImageFormat, ResourceTag, Vtf};

#[test]
fn rejects_invalid_signature_version_and_header_size() {
    let fixture = Fixture::new(3, 4, 4, ImageFormat::Rgba8888).build();

    let mut bad_signature = fixture.bytes.clone();
    bad_signature[0] = b'X';
    assert!(matches!(Vtf::from_bytes(&bad_signature).unwrap_err(), Error::InvalidSignature(_)));

    let mut bad_version = fixture.bytes.clone();
    write_u32(&mut bad_version, header::MINOR_VERSION, 6);
    assert!(matches!(
        Vtf::from_bytes(&bad_version).unwrap_err(),
        Error::UnsupportedVersion { major: 7, minor: 6 }
    ));

    let mut bad_header_size = fixture.bytes.clone();
    write_u32(&mut bad_header_size, header::HEADER_SIZE, 80);
    assert!(matches!(
        Vtf::from_bytes(&bad_header_size).unwrap_err(),
        Error::InvalidField { field: "header size", value: 80, .. }
    ));
}

#[test]
fn rejects_unknown_and_legacy_depth_image_formats() {
    let fixture = Fixture::new(5, 4, 4, ImageFormat::Rgba8888).build();
    let mut unknown = fixture.bytes.clone();
    write_i32(&mut unknown, header::IMAGE_FORMAT, 9999);
    assert!(matches!(
        Vtf::from_bytes(&unknown).unwrap_err(),
        Error::InvalidField { field: "image format", value: 9999, .. }
    ));

    let fixture = Fixture::new(4, 4, 4, ImageFormat::Rgba8888).build();
    for id in [30, 33] {
        let mut legacy_depth = fixture.bytes.clone();
        write_i32(&mut legacy_depth, header::IMAGE_FORMAT, id);
        assert!(matches!(
            Vtf::from_bytes(&legacy_depth).unwrap_err(),
            Error::UnsupportedFormat { field: "image format", id: found } if found == id
        ));
    }
}

#[test]
fn rejects_resources_outside_the_file() {
    let fixture = Fixture::new(3, 4, 4, ImageFormat::Rgba8888).build();
    let mut outside = fixture.bytes.clone();
    let outside_offset = outside.len() as u32 + 1;
    write_u32(&mut outside, header::RESOURCES + 4, outside_offset);
    assert!(matches!(
        Vtf::from_bytes(&outside).unwrap_err(),
        Error::OutOfBounds { context: "resource data", .. }
    ));

    let mut sized = Fixture::new(3, 4, 4, ImageFormat::Rgba8888);
    sized
        .data_resources
        .push((ResourceTag::KEY_VALUES_DATA, b"kv".to_vec()));
    let mut too_long = sized.build().bytes;
    let size_offset = u32::from_le_bytes(
        too_long[header::RESOURCES + 12..header::RESOURCES + 16]
            .try_into()
            .unwrap(),
    );
    write_u32(&mut too_long, size_offset as usize, u32::MAX);
    assert!(matches!(
        Vtf::from_bytes(&too_long).unwrap_err(),
        Error::OutOfBounds { context: "resource data", .. }
    ));
}

#[test]
fn rejects_missing_and_inline_image_resources() {
    let fixture = Fixture::new(3, 4, 4, ImageFormat::Rgba8888).build();

    let mut missing = fixture.bytes.clone();
    write_u32(&mut missing, header::RESOURCES, ResourceTag::CRC.0 | (2 << 24));
    assert!(matches!(
        Vtf::from_bytes(&missing).unwrap_err(),
        Error::MissingResource(ResourceTag::IMAGE_DATA)
    ));

    let mut inline = fixture.bytes.clone();
    write_u32(&mut inline, header::RESOURCES, ResourceTag::IMAGE_DATA.0 | (2 << 24));
    assert!(matches!(Vtf::from_bytes(&inline).unwrap_err(), Error::MissingResource(ResourceTag::IMAGE_DATA)));
}

#[test]
fn rejects_truncated_headers_resource_tables_and_image_data() {
    let legacy = Fixture::new(1, 4, 4, ImageFormat::Rgba8888).build();
    let truncated_image = &legacy.bytes[..legacy.bytes.len() - 1];
    assert!(matches!(
        Vtf::from_bytes(truncated_image).unwrap_err(),
        Error::OutOfBounds { context: "image data", .. }
    ));

    let modern = Fixture::new(3, 4, 4, ImageFormat::Rgba8888).build();
    assert!(matches!(
        Vtf::from_bytes(&modern.bytes[..84]).unwrap_err(),
        Error::OutOfBounds { context: "resource table", .. }
    ));

    for length in 0..64 {
        assert!(matches!(Vtf::from_bytes(&legacy.bytes[..length]).unwrap_err(), Error::OutOfBounds { .. }));
    }
}

#[test]
fn validates_dimensions_counts_and_subresource_indices() {
    let fixture = Fixture::new(5, 4, 4, ImageFormat::Rgba8888).build();

    let mut zero_width = fixture.bytes.clone();
    write_u16(&mut zero_width, header::WIDTH, 0);
    assert!(matches!(Vtf::from_bytes(&zero_width).unwrap_err(), Error::InvalidField { field: "width", .. }));

    let mut too_many_mips = fixture.bytes.clone();
    too_many_mips[header::MIP_COUNT] = 4;
    assert!(matches!(
        Vtf::from_bytes(&too_many_mips).unwrap_err(),
        Error::InvalidField { field: "mip count", value: 4, .. }
    ));

    let vtf = Vtf::from_bytes(&fixture.bytes).unwrap();
    for error in [
        vtf.subresource(1, 0, 0, 0).unwrap_err(),
        vtf.subresource(0, 1, 0, 0).unwrap_err(),
        vtf.subresource(0, 0, 1, 0).unwrap_err(),
        vtf.subresource(0, 0, 0, 1).unwrap_err(),
    ] {
        assert!(matches!(error, Error::InvalidIndex { index: 1, count: 1, .. }));
    }
}

#[test]
fn absurd_layouts_fail_on_bounds_without_allocating() {
    let mut bytes = Fixture::new(5, 1, 1, ImageFormat::Rgba8888)
        .build()
        .bytes;
    write_u16(&mut bytes, header::WIDTH, u16::MAX);
    write_u16(&mut bytes, header::HEIGHT, u16::MAX);
    write_u16(&mut bytes, header::FRAME_COUNT, u16::MAX);
    write_u16(&mut bytes, header::DEPTH, u16::MAX);
    bytes[header::MIP_COUNT] = 16;

    assert!(matches!(
        Vtf::from_bytes(&bytes).unwrap_err(),
        Error::OutOfBounds { context: "resource data", end: usize::MAX, .. }
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
        assert!(std::panic::catch_unwind(|| Vtf::from_bytes(&bytes)).is_ok());
    }
}
