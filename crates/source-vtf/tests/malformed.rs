mod common;

use common::{Fixture, write_i32, write_u32};
use source_vtf::{Error, ImageFormat, ResourceTag, Section, Vtf};

#[test]
fn rejects_invalid_signature_version_and_header_size() {
    let fixture = Fixture::new(3, 4, 4, ImageFormat::Rgba8888).build();

    let mut bad_signature = fixture.bytes.clone();
    bad_signature[0] = b'X';
    assert!(matches!(
        Vtf::from_bytes(&bad_signature).unwrap_err(),
        Error::InvalidSignature(_)
    ));

    let mut bad_version = fixture.bytes.clone();
    write_u32(&mut bad_version, 8, 6);
    assert!(matches!(
        Vtf::from_bytes(&bad_version).unwrap_err(),
        Error::UnsupportedVersion { major: 7, minor: 6 }
    ));

    let mut bad_header_size = fixture.bytes.clone();
    write_u32(&mut bad_header_size, 12, 80);
    assert!(matches!(
        Vtf::from_bytes(&bad_header_size).unwrap_err(),
        Error::InvalidHeaderSize { .. }
    ));
}

#[test]
fn rejects_unknown_and_legacy_depth_image_formats() {
    let fixture = Fixture::new(5, 4, 4, ImageFormat::Rgba8888).build();
    let mut unknown = fixture.bytes.clone();
    write_i32(&mut unknown, 52, 9999);
    assert!(matches!(
        Vtf::from_bytes(&unknown).unwrap_err(),
        Error::UnknownImageFormat {
            field: "high-resolution",
            value: 9999,
            ..
        }
    ));

    let fixture = Fixture::new(4, 4, 4, ImageFormat::Rgba8888).build();
    let mut legacy_depth = fixture.bytes.clone();
    write_i32(&mut legacy_depth, 52, 33);
    assert!(matches!(
        Vtf::from_bytes(&legacy_depth).unwrap_err(),
        Error::UnsupportedImageFormat { value: 33, .. }
    ));
}

#[test]
fn rejects_resource_offsets_outside_the_file_or_inside_the_header() {
    let fixture = Fixture::new(3, 4, 4, ImageFormat::Rgba8888).build();

    let mut outside = fixture.bytes.clone();
    let outside_offset = outside.len() as u32 + 1;
    write_u32(&mut outside, 84, outside_offset);
    assert!(matches!(
        Vtf::from_bytes(&outside).unwrap_err(),
        Error::RangeOutOfBounds {
            section: Section::ResourceData,
            ..
        }
    ));

    let mut inside = fixture.bytes.clone();
    write_u32(&mut inside, 84, 80);
    assert!(matches!(
        Vtf::from_bytes(&inside).unwrap_err(),
        Error::ResourceInsideHeader { .. }
    ));
}

#[test]
fn rejects_missing_duplicate_and_inline_image_resources() {
    let fixture = Fixture::new(3, 4, 4, ImageFormat::Rgba8888).build();

    let mut missing = fixture.bytes.clone();
    write_u32(&mut missing, 80, ResourceTag::KEY_VALUES_DATA.value());
    assert!(matches!(
        Vtf::from_bytes(&missing).unwrap_err(),
        Error::MissingResource { tag } if tag == ResourceTag::IMAGE_DATA.value()
    ));

    let mut duplicate_fixture = Fixture::new(3, 4, 4, ImageFormat::Rgba8888);
    duplicate_fixture
        .data_resources
        .push((ResourceTag::IMAGE_DATA, b"duplicate".to_vec()));
    assert!(matches!(
        Vtf::from_bytes(&duplicate_fixture.build().bytes).unwrap_err(),
        Error::DuplicateResource { tag } if tag == ResourceTag::IMAGE_DATA.value()
    ));

    let mut inline = fixture.bytes.clone();
    write_u32(
        &mut inline,
        80,
        ResourceTag::IMAGE_DATA.value() | (2u32 << 24),
    );
    assert!(matches!(
        Vtf::from_bytes(&inline).unwrap_err(),
        Error::InlineTextureData { tag } if tag == ResourceTag::IMAGE_DATA.value()
    ));
}

#[test]
fn rejects_truncated_headers_resource_tables_and_image_data() {
    let legacy = Fixture::new(1, 4, 4, ImageFormat::Rgba8888).build();
    let truncated_image = &legacy.bytes[..legacy.bytes.len() - 1];
    assert!(matches!(
        Vtf::from_bytes(truncated_image).unwrap_err(),
        Error::RangeOutOfBounds {
            section: Section::ImageData,
            ..
        }
    ));

    let modern = Fixture::new(3, 4, 4, ImageFormat::Rgba8888).build();
    assert!(matches!(
        Vtf::from_bytes(&modern.bytes[..84]).unwrap_err(),
        Error::UnexpectedEof {
            section: Section::Header,
            ..
        }
    ));

    for length in 0..64 {
        assert!(matches!(
            Vtf::from_bytes(&legacy.bytes[..length]).unwrap_err(),
            Error::UnexpectedEof { .. }
        ));
    }
}

#[test]
fn validates_dimensions_counts_and_subresource_indices() {
    let fixture = Fixture::new(5, 4, 4, ImageFormat::Rgba8888).build();

    let mut zero_width = fixture.bytes.clone();
    zero_width[16..18].copy_from_slice(&0u16.to_le_bytes());
    assert!(matches!(
        Vtf::from_bytes(&zero_width).unwrap_err(),
        Error::InvalidField { field: "width", .. }
    ));

    let mut too_many_mips = fixture.bytes.clone();
    too_many_mips[56] = 4;
    assert!(matches!(
        Vtf::from_bytes(&too_many_mips).unwrap_err(),
        Error::InvalidField {
            field: "mip count",
            ..
        }
    ));

    let vtf = Vtf::from_bytes(&fixture.bytes).unwrap();
    for error in [
        vtf.subresource(1, 0, 0, 0).unwrap_err(),
        vtf.subresource(0, 1, 0, 0).unwrap_err(),
        vtf.subresource(0, 0, 1, 0).unwrap_err(),
        vtf.subresource(0, 0, 0, 1).unwrap_err(),
    ] {
        assert!(matches!(error, Error::SubresourceOutOfRange { .. }));
    }
}

#[test]
fn large_declared_layouts_fail_without_allocating_the_payload() {
    let mut fixture = Fixture::new(5, u16::MAX, u16::MAX, ImageFormat::Rgba8888);
    fixture.depth = u16::MAX;
    fixture.frame_count = u16::MAX;
    fixture.mip_count = 16;

    // Building the payload would be enormous, so mutate a small valid fixture's
    // metadata and let the checked layout calculation reject its tiny resource.
    let mut bytes = Fixture::new(5, 1, 1, ImageFormat::Rgba8888).build().bytes;
    bytes[16..18].copy_from_slice(&fixture.width.to_le_bytes());
    bytes[18..20].copy_from_slice(&fixture.height.to_le_bytes());
    bytes[24..26].copy_from_slice(&fixture.frame_count.to_le_bytes());
    bytes[56] = fixture.mip_count;
    bytes[63..65].copy_from_slice(&fixture.depth.to_le_bytes());

    assert!(matches!(
        Vtf::from_bytes(&bytes).unwrap_err(),
        Error::DataTooShort {
            section: Section::ImageData,
            ..
        } | Error::ArithmeticOverflow { .. }
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
