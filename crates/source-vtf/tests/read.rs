mod common;

use std::fs;

use common::Fixture;
use source_vtf::{ImageFormat, ResourceLocation, ResourceTag, TextureFlags, Version, Vtf};
use tempfile::tempdir;

#[test]
fn reads_legacy_thumbnail_and_smallest_first_mips() {
    let mut fixture = Fixture::new(1, 8, 4, ImageFormat::Rgba8888);
    fixture.mip_count = 4;
    fixture.thumbnail = Some((4, 4));
    let built = fixture.build();

    let vtf = Vtf::from_bytes(&built.bytes).unwrap();
    assert_eq!(vtf.header().version(), Version::V7_1);
    assert_eq!(vtf.header().header_size(), 64);
    assert_eq!(vtf.header().reflectivity(), [0.25, 0.5, 0.75]);
    assert_eq!(vtf.header().bump_map_scale(), 1.5);
    assert_eq!(vtf.mip_count(), 4);

    let thumbnail = vtf.thumbnail().unwrap();
    assert_eq!(thumbnail.format(), ImageFormat::Dxt1);
    assert_eq!((thumbnail.width(), thumbnail.height()), (4, 4));
    assert_eq!(thumbnail.data(), built.thumbnail.as_deref().unwrap());

    assert_images(&vtf, &built.images);
    assert_eq!(built.images[0].mip, 3);
    assert_eq!(built.images.last().unwrap().mip, 0);
}

#[test]
fn addresses_frames_and_halved_volume_slices() {
    let mut fixture = Fixture::new(2, 4, 2, ImageFormat::Rgba8888);
    fixture.depth = 4;
    fixture.frame_count = 2;
    fixture.mip_count = 3;
    let built = fixture.build();

    let vtf = Vtf::from_bytes(&built.bytes).unwrap();
    assert_eq!(vtf.header().version(), Version::V7_2);
    assert_eq!(vtf.header().header_size(), 80);
    assert_eq!(vtf.depth(), 4);
    assert_eq!(vtf.mip_dimensions(0).unwrap(), (4, 2, 4));
    assert_eq!(vtf.mip_dimensions(1).unwrap(), (2, 1, 2));
    assert_eq!(vtf.mip_dimensions(2).unwrap(), (1, 1, 1));
    assert_images(&vtf, &built.images);
}

#[test]
fn parses_out_of_order_resource_offsets_and_inline_data() {
    let mut fixture = Fixture::new(3, 4, 4, ImageFormat::Dxt5);
    fixture.mip_count = 3;
    fixture.thumbnail = Some((4, 4));
    fixture
        .inline_resources
        .push((ResourceTag::CRC, 0x1234_5678u32.to_le_bytes()));
    fixture
        .data_resources
        .push((ResourceTag::KEY_VALUES_DATA, b"{\n\tmetadata\n}\0".to_vec()));
    let built = fixture.build();

    let vtf = Vtf::from_bytes(&built.bytes).unwrap();
    assert_eq!(vtf.header().version(), Version::V7_3);
    assert_eq!(vtf.header().resource_count(), 4);
    assert_eq!(vtf.resources()[0].tag(), ResourceTag::IMAGE_DATA);
    assert!(matches!(
        vtf.resource(ResourceTag::CRC).unwrap().location(),
        ResourceLocation::Inline(_)
    ));
    assert_eq!(
        vtf.resource_data(ResourceTag::CRC).unwrap(),
        0x1234_5678u32.to_le_bytes()
    );
    assert_eq!(
        vtf.resource_data(ResourceTag::KEY_VALUES_DATA).unwrap(),
        b"{\n\tmetadata\n}\0"
    );
    assert_images(&vtf, &built.images);
}

#[test]
fn supports_resource_vtfs_without_a_thumbnail() {
    let fixture = Fixture::new(5, 7, 5, ImageFormat::Dxt1);
    let built = fixture.build();
    let vtf = Vtf::from_bytes(&built.bytes).unwrap();

    assert!(vtf.thumbnail().is_none());
    assert!(vtf.resource(ResourceTag::THUMBNAIL_DATA).is_none());
    let image = vtf.subresource(0, 0, 0, 0).unwrap();
    assert_eq!(image.len(), 32);
    assert_eq!(image.data(), built.images[0].data);
}

#[test]
fn follows_the_versioned_legacy_sphere_map_rule() {
    for (minor, start_frame, expected_faces) in
        [(0, 0, 6), (1, 0, 7), (1, u16::MAX, 6), (4, 0, 7), (5, 0, 6)]
    {
        let mut fixture = Fixture::new(minor, 1, 1, ImageFormat::Rgba8888);
        fixture.flags = TextureFlags::ENVMAP;
        fixture.start_frame = start_frame;
        let built = fixture.build();
        let vtf = Vtf::from_bytes(&built.bytes).unwrap();

        assert_eq!(vtf.face_count(), expected_faces, "VTF 7.{minor}");
        assert_eq!(built.images.len(), usize::from(expected_faces));
        assert_images(&vtf, &built.images);
    }
}

#[test]
fn open_owns_the_file_while_from_bytes_keeps_borrowed_views() {
    let fixture = Fixture::new(5, 2, 2, ImageFormat::Rgba8888).build();
    let borrowed = Vtf::from_bytes(&fixture.bytes).unwrap();
    let raw = borrowed.subresource(0, 0, 0, 0).unwrap();
    let input_range = fixture.bytes.as_ptr_range();
    assert!(input_range.contains(&raw.data().as_ptr()));

    let directory = tempdir().unwrap();
    let path = directory.path().join("owned.vtf");
    fs::write(&path, &fixture.bytes).unwrap();
    let owned = Vtf::open(&path).unwrap();
    fs::remove_file(path).unwrap();

    assert_eq!(
        owned.subresource(0, 0, 0, 0).unwrap().data(),
        fixture.images[0].data
    );
}

fn assert_images(vtf: &Vtf<'_>, expected: &[common::ExpectedImage]) {
    for image in expected {
        let actual = vtf
            .subresource(image.mip, image.frame, image.face, image.slice)
            .unwrap();
        assert_eq!(
            (actual.width(), actual.height()),
            (image.width, image.height)
        );
        assert_eq!(actual.data(), image.data);
    }
}
