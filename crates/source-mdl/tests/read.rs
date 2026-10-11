mod common;

use common::{mdl_header, read_i32, studio_fixture, write_i32, write_section};
use source_mdl::{GeometryOptions, Mdl, StudioModel, Vtx, Vvd};

#[test]
fn parses_mdl_without_companion_files() {
    let fixture = studio_fixture(44);
    let mdl = Mdl::from_bytes(&fixture.mdl).unwrap();

    assert_eq!(mdl.header.version, 44);
    assert_eq!(mdl.header.name, "models/test/sample.mdl");
    assert_eq!(mdl.bones.len(), 1);
    assert_eq!(mdl.bones[0].name, "root");
    assert_eq!(mdl.bones[0].parent, None);
    assert_eq!(mdl.bones[0].surface_property, "default");
    assert_eq!(mdl.bone_controllers[0].bone, 0);
    assert_eq!(mdl.hitbox_sets[0].hitboxes[0].name, "body");
    assert_eq!(mdl.materials[0].name, "material_a");
    assert_eq!(mdl.material_directories, ["models/test"]);
    assert_eq!(mdl.skins, [vec![0, 1], vec![1, 0]]);
    assert_eq!(mdl.body_parts[0].models[0].meshes[0].id, 7);
    assert_eq!(mdl.material_path_candidates(0).unwrap(), ["models/test/material_a"]);
}

#[test]
fn parses_vvd_fixups_and_vtx_material_replacements() {
    let fixture = studio_fixture(44);
    let vvd = Vvd::from_bytes(&fixture.vvd).unwrap();
    let vtx = Vtx::from_bytes(&fixture.vtx, 44).unwrap();

    let first_axis = |lod| -> Vec<f32> {
        vvd.lod_vertices(lod)
            .unwrap()
            .iter()
            .map(|vertex| vertex.position[0])
            .collect()
    };
    assert_eq!(vvd.lod_count, 2);
    assert_eq!(first_axis(0), [0.0, 1.0, 2.0, 3.0]);
    assert_eq!(first_axis(1), [1.0, 2.0, 3.0]);
    assert_eq!(vvd.vertices[2].position, [2.0, 2.25, 2.5]);
    assert_eq!(vtx.lod_count, 2);
    assert!(vtx.material_replacements[0].is_empty());
    assert_eq!(vtx.material_replacements[1][0].material, 1);
    assert_eq!(vtx.material_replacements[1][0].name, "replacement/lod1");
}

#[test]
fn extracts_lod_geometry_without_hidden_coordinate_conversion() {
    let fixture = studio_fixture(44);
    let model = StudioModel::from_bytes(&fixture.mdl, &fixture.vvd, &fixture.vtx).unwrap();

    let lod0 = model
        .geometry(&GeometryOptions::default())
        .unwrap();
    assert_eq!(lod0.vertices.len(), 4);
    assert_eq!(lod0.meshes[0].indices, [0, 1, 2, 0, 2, 3]);
    assert_eq!(lod0.vertices[0].position, [0.0, 0.25, 0.5]);
    assert_eq!(lod0.meshes[0].material.path_candidates, ["models/test/material_a"]);

    let lod1 = model
        .geometry(&GeometryOptions { lod: 1, skin: 1, ..Default::default() })
        .unwrap();
    assert_eq!(lod1.vertices.len(), 3);
    assert_eq!(lod1.vertices[0].position, [1.0, 1.25, 1.5]);
    assert_eq!(lod1.meshes[0].indices, [0, 1, 2]);
    assert_eq!(lod1.meshes[0].material.index, 1);
    assert_eq!(lod1.meshes[0].material.path_candidates, ["replacement/lod1"]);
}

#[test]
fn parses_version_49_extended_strip_records() {
    let fixture = studio_fixture(49);
    let model = StudioModel::from_bytes(&fixture.mdl, &fixture.vvd, &fixture.vtx).unwrap();

    assert_eq!(model.mdl().header.version, 49);
    let lod1 = model
        .geometry(&GeometryOptions { lod: 1, ..Default::default() })
        .unwrap();
    assert_eq!(lod1.meshes[0].indices, [0, 1, 2]);
}

#[test]
fn material_mapping_does_not_require_geometry_extraction() {
    let fixture = studio_fixture(44);
    let model = StudioModel::from_bytes(&fixture.mdl, &fixture.vvd, &fixture.vtx).unwrap();
    let materials = model
        .material_references(&GeometryOptions { lod: 1, skin: 1, ..Default::default() })
        .unwrap();

    assert_eq!(materials.len(), 1);
    assert_eq!(materials[0].index, 1);
    assert_eq!(materials[0].path_candidates, ["replacement/lod1"]);
}

#[test]
fn opens_companions_with_different_casing() {
    let fixture = studio_fixture(44);
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("sample.mdl"), &fixture.mdl).unwrap();
    std::fs::write(directory.path().join("Sample.VVD"), &fixture.vvd).unwrap();
    std::fs::write(directory.path().join("SAMPLE.dx90.vtx"), &fixture.vtx).unwrap();

    let model = StudioModel::open(directory.path().join("sample.mdl")).unwrap();
    assert_eq!(model.mdl().bones[0].name, "root");
}

#[test]
fn flips_triangle_winding_to_ccw() {
    let fixture = studio_fixture(44);
    let model = StudioModel::from_bytes(&fixture.mdl, &fixture.vvd, &fixture.vtx).unwrap();

    let ccw = model
        .geometry(&GeometryOptions::default())
        .unwrap()
        .into_ccw();
    assert_eq!(ccw.meshes[0].indices, [0, 2, 1, 0, 3, 2]);
}

#[test]
fn parses_mdl_with_empty_hitbox_set_and_past_eof_offset() {
    let fixture = studio_fixture(44);
    let mut mdl = fixture.mdl.clone();

    let hitbox_set = read_i32(&mdl, mdl_header::HITBOX_SETS + 4) as usize;
    write_i32(&mut mdl, hitbox_set + 4, 0);
    write_i32(&mut mdl, hitbox_set + 8, 99999);

    let parsed = Mdl::from_bytes(&mdl).unwrap();
    assert_eq!(parsed.hitbox_sets[0].hitboxes.len(), 0);
}

#[test]
fn parses_mdl_with_hitbox_name_offset_past_eof() {
    let fixture = studio_fixture(44);
    let mut mdl = fixture.mdl.clone();

    let hitbox_set = read_i32(&mdl, mdl_header::HITBOX_SETS + 4) as usize;
    let hitbox = hitbox_set + read_i32(&mdl, hitbox_set + 8) as usize;
    // szhitboxnameindex past EOF, as in ~7.5% of Portal 2 release models
    write_i32(&mut mdl, hitbox + 32, 99999);

    let parsed = Mdl::from_bytes(&mdl).unwrap();
    assert_eq!(parsed.hitbox_sets[0].hitboxes.len(), 1);
    assert_eq!(parsed.hitbox_sets[0].hitboxes[0].name, "");
}

#[test]
fn malformed_animation_tracks_do_not_block_the_model() {
    let fixture = studio_fixture(44);
    let mut mdl = fixture.mdl.clone();

    // One mstudioanimdesc_t with a single frame and its track right after it
    let animation = mdl.len();
    mdl.resize(animation + 104, 0);
    write_section(&mut mdl, mdl_header::ANIMATIONS, 1, animation);
    write_i32(&mut mdl, animation + 16, 1); // numframes
    write_i32(&mut mdl, animation + 56, 100); // animindex
    mdl[animation + 100] = 9; // track bone, but the model has a single bone
    let length = mdl.len() as i32;
    write_i32(&mut mdl, mdl_header::LENGTH, length);

    let parsed = Mdl::from_bytes(&mdl).unwrap();
    assert!(parsed.animations[0].tracks.is_empty());
    assert_eq!(parsed.animations[0].unsupported, Some("malformed animation tracks"));
    assert!(StudioModel::from_bytes(&mdl, &fixture.vvd, &fixture.vtx).is_ok());
}
