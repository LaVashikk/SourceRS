// ==========================================================
//     THIS PIECE OF CRAP AND EVERY TESTS GENERATED           cause im lazy do that
// ==========================================================

#![allow(dead_code)]

pub const CHECKSUM: u32 = 0x1234_5678;

/// Byte offsets of `studiohdr_t` fields; a section offset is a `(count, offset)` pair
pub mod mdl_header {
    pub const LENGTH: usize = 76;
    pub const BONES: usize = 156;
    pub const BONE_CONTROLLERS: usize = 164;
    pub const HITBOX_SETS: usize = 172;
    pub const ANIMATIONS: usize = 180;
    pub const MATERIALS: usize = 204;
    pub const MATERIAL_DIRECTORIES: usize = 212;
    /// `numskinref`, then the `(numskinfamilies, skinindex)` pair
    pub const SKIN_REFERENCES: usize = 220;
    pub const BODY_PARTS: usize = 232;
}

/// Byte offsets of `vertexFileHeader_t` fields
pub mod vvd_header {
    pub const CHECKSUM: usize = 8;
    pub const LOD_COUNT: usize = 12;
    pub const LOD_VERTEX_COUNTS: usize = 16;
    pub const FIXUPS: usize = 48;
    pub const VERTEX_OFFSET: usize = 56;
    pub const TANGENT_OFFSET: usize = 60;
}

/// Byte offsets of the VTX `FileHeader_t` fields
pub mod vtx_header {
    pub const CHECKSUM: usize = 16;
    pub const LOD_COUNT: usize = 20;
    pub const REPLACEMENT_LISTS: usize = 24;
    pub const BODY_PARTS: usize = 28;
}

#[derive(Debug, Clone)]
pub struct Fixture {
    pub mdl: Vec<u8>,
    pub vvd: Vec<u8>,
    pub vtx: Vec<u8>,
}

/// A minimal valid MDL/VVD/VTX set: one bone, two materials and skins, one mesh with two LODs.
/// Built byte by byte because Valve assets cannot be shipped with the tests
pub fn studio_fixture(version: i32) -> Fixture {
    Fixture { mdl: build_mdl(version), vvd: build_vvd(), vtx: build_vtx(version) }
}

fn build_mdl(version: i32) -> Vec<u8> {
    let mut bytes = vec![0; 240];
    bytes[0..4].copy_from_slice(b"IDST");
    write_i32(&mut bytes, 4, version);
    write_u32(&mut bytes, 8, CHECKSUM);
    write_fixed(&mut bytes, 12, 64, b"models/test/sample.mdl");
    write_f32(&mut bytes, 80, 1.0);
    write_f32(&mut bytes, 96, -1.0);
    write_f32(&mut bytes, 108, 1.0);
    write_f32(&mut bytes, 120, -2.0);
    write_f32(&mut bytes, 132, 2.0);

    let bone = push_zeros(&mut bytes, 216);
    write_section(&mut bytes, mdl_header::BONES, 1, bone);
    write_i32(&mut bytes, bone + 4, -1);
    write_i32(&mut bytes, bone + 8, 0);
    for slot in 1..6 {
        write_i32(&mut bytes, bone + 8 + slot * 4, -1);
    }
    write_f32(&mut bytes, bone + 32, 1.0);
    write_f32(&mut bytes, bone + 44 + 12, 1.0);
    write_f32(&mut bytes, bone + 96, 1.0);
    write_f32(&mut bytes, bone + 96 + 20, 1.0);
    write_f32(&mut bytes, bone + 96 + 40, 1.0);
    write_f32(&mut bytes, bone + 144 + 12, 1.0);
    let bone_name = push_c_string(&mut bytes, b"root");
    let surface = push_c_string(&mut bytes, b"default");
    write_i32(&mut bytes, bone, (bone_name - bone) as i32);
    write_i32(&mut bytes, bone + 176, (surface - bone) as i32);

    let controller = push_zeros(&mut bytes, 56);
    write_section(&mut bytes, mdl_header::BONE_CONTROLLERS, 1, controller);
    write_i32(&mut bytes, controller, 0);
    write_i32(&mut bytes, controller + 4, 1);
    write_f32(&mut bytes, controller + 8, -1.0);
    write_f32(&mut bytes, controller + 12, 1.0);

    let hitbox_set = push_zeros(&mut bytes, 12);
    let hitbox = push_zeros(&mut bytes, 68);
    let hitbox_set_name = push_c_string(&mut bytes, b"default");
    let hitbox_name = push_c_string(&mut bytes, b"body");
    write_section(&mut bytes, mdl_header::HITBOX_SETS, 1, hitbox_set);
    write_i32(&mut bytes, hitbox_set, (hitbox_set_name - hitbox_set) as i32);
    write_i32(&mut bytes, hitbox_set + 4, 1);
    write_i32(&mut bytes, hitbox_set + 8, (hitbox - hitbox_set) as i32);
    write_i32(&mut bytes, hitbox, 0);
    write_i32(&mut bytes, hitbox + 4, 1);
    write_f32(&mut bytes, hitbox + 8, -1.0);
    write_f32(&mut bytes, hitbox + 20, 1.0);
    write_i32(&mut bytes, hitbox + 32, (hitbox_name - hitbox) as i32);

    let materials = push_zeros(&mut bytes, 2 * 64);
    let material_a = push_c_string(&mut bytes, b"material_a");
    let material_b = push_c_string(&mut bytes, b"material_b");
    write_section(&mut bytes, mdl_header::MATERIALS, 2, materials);
    write_i32(&mut bytes, materials, (material_a - materials) as i32);
    write_i32(&mut bytes, materials + 64, (material_b - (materials + 64)) as i32);

    let directory_table = push_zeros(&mut bytes, 4);
    let directory = push_c_string(&mut bytes, b"models/test");
    write_section(&mut bytes, mdl_header::MATERIAL_DIRECTORIES, 1, directory_table);
    write_i32(&mut bytes, directory_table, directory as i32);

    let skins = bytes.len();
    push_i16(&mut bytes, 0);
    push_i16(&mut bytes, 1);
    push_i16(&mut bytes, 1);
    push_i16(&mut bytes, 0);
    write_i32(&mut bytes, mdl_header::SKIN_REFERENCES, 2);
    write_section(&mut bytes, mdl_header::SKIN_REFERENCES + 4, 2, skins);

    let body_part = push_zeros(&mut bytes, 16);
    let model = push_zeros(&mut bytes, 148);
    let mesh = push_zeros(&mut bytes, 116);
    let body_name = push_c_string(&mut bytes, b"body");
    write_section(&mut bytes, mdl_header::BODY_PARTS, 1, body_part);
    write_i32(&mut bytes, body_part, (body_name - body_part) as i32);
    write_i32(&mut bytes, body_part + 4, 1);
    write_i32(&mut bytes, body_part + 8, 1);
    write_i32(&mut bytes, body_part + 12, (model - body_part) as i32);

    write_fixed(&mut bytes, model, 64, b"sample");
    write_f32(&mut bytes, model + 68, 2.0);
    write_i32(&mut bytes, model + 72, 1);
    write_i32(&mut bytes, model + 76, (mesh - model) as i32);
    write_i32(&mut bytes, model + 80, 4);
    write_i32(&mut bytes, model + 84, 0);

    write_i32(&mut bytes, mesh, 0);
    write_i32(&mut bytes, mesh + 8, 4);
    write_i32(&mut bytes, mesh + 12, 0);
    write_i32(&mut bytes, mesh + 32, 7);
    write_f32(&mut bytes, mesh + 36, 0.5);
    write_i32(&mut bytes, mesh + 52, 4);
    write_i32(&mut bytes, mesh + 56, 3);
    for lod in 2..8 {
        write_i32(&mut bytes, mesh + 52 + lod * 4, 3);
    }

    let length = bytes.len() as i32;
    write_i32(&mut bytes, mdl_header::LENGTH, length);
    bytes
}

fn build_vvd() -> Vec<u8> {
    let mut bytes = vec![0; 64];
    bytes[0..4].copy_from_slice(b"IDSV");
    write_i32(&mut bytes, 4, 4);
    write_u32(&mut bytes, vvd_header::CHECKSUM, CHECKSUM);
    write_i32(&mut bytes, vvd_header::LOD_COUNT, 2);
    write_i32(&mut bytes, vvd_header::LOD_VERTEX_COUNTS, 4);
    write_i32(&mut bytes, vvd_header::LOD_VERTEX_COUNTS + 4, 3);

    let fixups = bytes.len();
    push_i32(&mut bytes, 0);
    push_i32(&mut bytes, 0);
    push_i32(&mut bytes, 1);
    push_i32(&mut bytes, 1);
    push_i32(&mut bytes, 1);
    push_i32(&mut bytes, 3);
    let vertices = bytes.len();
    for index in 0..4 {
        push_f32(&mut bytes, 1.0);
        push_f32(&mut bytes, 0.0);
        push_f32(&mut bytes, 0.0);
        bytes.extend_from_slice(&[0, 0, 0, 1]);
        push_f32(&mut bytes, index as f32);
        push_f32(&mut bytes, index as f32 + 0.25);
        push_f32(&mut bytes, index as f32 + 0.5);
        push_f32(&mut bytes, 0.0);
        push_f32(&mut bytes, 0.0);
        push_f32(&mut bytes, 1.0);
        push_f32(&mut bytes, index as f32 / 4.0);
        push_f32(&mut bytes, index as f32 / 8.0);
    }
    let tangents = bytes.len();
    for _ in 0..4 {
        push_f32(&mut bytes, 1.0);
        push_f32(&mut bytes, 0.0);
        push_f32(&mut bytes, 0.0);
        push_f32(&mut bytes, 1.0);
    }
    write_section(&mut bytes, vvd_header::FIXUPS, 2, fixups);
    write_i32(&mut bytes, vvd_header::VERTEX_OFFSET, vertices as i32);
    write_i32(&mut bytes, vvd_header::TANGENT_OFFSET, tangents as i32);
    bytes
}

fn build_vtx(mdl_version: i32) -> Vec<u8> {
    let mut bytes = vec![0; 36];
    // version, vertCacheSize, maxBonesPerStrip, maxBonesPerTri, maxBonesPerVert
    write_i32(&mut bytes, 0, 7);
    write_i32(&mut bytes, 4, 24);
    write_u16(&mut bytes, 8, 53);
    write_u16(&mut bytes, 10, 9);
    write_i32(&mut bytes, 12, 3);
    write_u32(&mut bytes, vtx_header::CHECKSUM, CHECKSUM);
    write_i32(&mut bytes, vtx_header::LOD_COUNT, 2);

    let replacement_lists = push_zeros(&mut bytes, 16);
    write_i32(&mut bytes, vtx_header::REPLACEMENT_LISTS, replacement_lists as i32);
    write_i32(&mut bytes, replacement_lists, 0);
    write_i32(&mut bytes, replacement_lists + 4, -4);
    let replacement = push_zeros(&mut bytes, 6);
    let replacement_name = push_c_string(&mut bytes, b"replacement/lod1");
    write_i32(&mut bytes, replacement_lists + 8, 1);
    write_i32(&mut bytes, replacement_lists + 12, (replacement - (replacement_lists + 8)) as i32);
    write_i16(&mut bytes, replacement, 1);
    write_i32(&mut bytes, replacement + 2, (replacement_name - replacement) as i32);

    let body = push_zeros(&mut bytes, 8);
    let model = push_zeros(&mut bytes, 8);
    let lods = push_zeros(&mut bytes, 2 * 12);
    write_section(&mut bytes, vtx_header::BODY_PARTS, 1, body);
    write_i32(&mut bytes, body, 1);
    write_i32(&mut bytes, body + 4, (model - body) as i32);
    write_i32(&mut bytes, model, 2);
    write_i32(&mut bytes, model + 4, (lods - model) as i32);

    append_lod(&mut bytes, lods, 4, &[0, 1, 2, 0, 2, 3], mdl_version);
    append_lod(&mut bytes, lods + 12, 3, &[0, 1, 2], mdl_version);
    bytes
}

fn append_lod(bytes: &mut Vec<u8>, lod: usize, vertex_count: usize, indices: &[u16], mdl_version: i32) {
    let mesh = push_zeros(bytes, 9);
    write_i32(bytes, lod, 1);
    write_i32(bytes, lod + 4, (mesh - lod) as i32);
    write_f32(bytes, lod + 8, if vertex_count == 4 { 0.0 } else { 10.0 });

    let group_size = if mdl_version >= 49 { 33 } else { 25 };
    let strip_size = if mdl_version >= 49 { 35 } else { 27 };
    let group = push_zeros(bytes, group_size);
    write_i32(bytes, mesh, 1);
    write_i32(bytes, mesh + 4, (group - mesh) as i32);

    let vertices = push_zeros(bytes, vertex_count * 9);
    for index in 0..vertex_count {
        let base = vertices + index * 9;
        bytes[base + 3] = 1;
        write_u16(bytes, base + 4, index as u16);
    }
    let index_base = bytes.len();
    for &index in indices {
        push_u16(bytes, index);
    }
    let strip = push_zeros(bytes, strip_size);
    write_i32(bytes, group, vertex_count as i32);
    write_i32(bytes, group + 4, (vertices - group) as i32);
    write_i32(bytes, group + 8, indices.len() as i32);
    write_i32(bytes, group + 12, (index_base - group) as i32);
    write_i32(bytes, group + 16, 1);
    write_i32(bytes, group + 20, (strip - group) as i32);

    write_i32(bytes, strip, indices.len() as i32);
    write_i32(bytes, strip + 4, 0);
    write_i32(bytes, strip + 8, vertex_count as i32);
    write_i32(bytes, strip + 12, 0);
    write_i16(bytes, strip + 16, 1);
    bytes[strip + 18] = 1;
}

/// Writes a `(count, offset)` pair
pub fn write_section(bytes: &mut [u8], at: usize, count: i32, offset: usize) {
    write_i32(bytes, at, count);
    write_i32(bytes, at + 4, offset as i32);
}

pub fn read_i32(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

pub fn write_i16(bytes: &mut [u8], offset: usize, value: i16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

pub fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

pub fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_f32(bytes: &mut [u8], offset: usize, value: f32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_fixed(bytes: &mut [u8], offset: usize, length: usize, value: &[u8]) {
    assert!(value.len() < length);
    bytes[offset..offset + value.len()].copy_from_slice(value);
}

fn push_zeros(bytes: &mut Vec<u8>, length: usize) -> usize {
    let offset = bytes.len();
    bytes.resize(offset + length, 0);
    offset
}

fn push_c_string(bytes: &mut Vec<u8>, value: &[u8]) -> usize {
    let offset = bytes.len();
    bytes.extend_from_slice(value);
    bytes.push(0);
    offset
}

fn push_i16(bytes: &mut Vec<u8>, value: i16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_i32(bytes: &mut Vec<u8>, value: i32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_f32(bytes: &mut Vec<u8>, value: f32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
