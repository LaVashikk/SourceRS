//! Optimized strip data: the `OptimizedModel` structs of the SDK's `public/optimize.h`
//!
//! Records are packed (`pragma pack(1)`).
//! The file version is 7 for every MDL version this crate reads.
//! MDL 49 models append two topology fields to strip groups and strips, and the VTX itself never says so.

use bitflags::bitflags;

use crate::cursor::{Reader, index, u16_at};
use crate::{Error, FileKind, MAX_LODS, Result};

const VERSION: i32 = 7; // OPTIMIZED_MODEL_FILE_VERSION
const REPLACEMENT_LIST_SIZE: usize = 8; // MaterialReplacementListHeader_t
const REPLACEMENT_SIZE: usize = 6; // MaterialReplacementHeader_t
const BODY_PART_SIZE: usize = 8; // BodyPartHeader_t
const MODEL_SIZE: usize = 8; // ModelHeader_t
const LOD_SIZE: usize = 12; // ModelLODHeader_t
const MESH_SIZE: usize = 9; // MeshHeader_t
const STRIP_GROUP_SIZE: usize = 25; // StripGroupHeader_t
const STRIP_SIZE: usize = 27; // StripHeader_t
const VERTEX_SIZE: usize = 9; // Vertex_t
const V49_EXTRA_SIZE: usize = 8; // numTopologyIndices, topologyOffset

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct MeshFlags: u8 {
        const TEETH = 1 << 0;
        const EYES = 1 << 1;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct StripGroupFlags: u8 {
        const FLEXED = 1 << 0;
        const HARDWARE_SKINNED = 1 << 1;
        const DELTA_FLEXED = 1 << 2;
        const SUPPRESS_HARDWARE_MORPH = 1 << 3;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct StripFlags: u8 {
        const TRIANGLE_LIST = 1 << 0;
        const TRIANGLE_STRIP = 1 << 1;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vtx {
    pub checksum: u32,
    pub lod_count: usize,
    /// Per LOD
    pub material_replacements: Vec<Vec<MaterialReplacement>>,
    pub body_parts: Vec<BodyPart>,
}

/// A QC `$lod` `replacematerial`: MDL material `material` is drawn as `name` at that LOD
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MaterialReplacement {
    pub material: usize,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BodyPart {
    pub models: Vec<Model>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Model {
    pub lods: Vec<Lod>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Lod {
    pub meshes: Vec<Mesh>,
    pub switch_distance: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Mesh {
    pub strip_groups: Vec<StripGroup>,
    pub flags: MeshFlags,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StripGroup {
    pub vertices: Vec<Vertex>,
    /// Indices into `vertices`
    pub indices: Vec<u16>,
    pub strips: Vec<Strip>,
    pub flags: StripGroupFlags,
}

/// A slice of its group's `indices` and `vertices`
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Strip {
    pub index_offset: usize,
    pub index_count: usize,
    pub vertex_offset: usize,
    pub vertex_count: usize,
    pub bone_count: i16,
    pub flags: StripFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vertex {
    pub bone_weight_indices: [u8; 3],
    pub bone_count: u8,
    /// Vertex of the MDL mesh at this LOD
    pub original_mesh_vertex: usize,
    pub bone_ids: [i8; 3],
}

impl Vtx {
    /// `mdl_version` selects the strip layout, which the VTX file does not record itself
    pub fn from_bytes(bytes: &[u8], mdl_version: i32) -> Result<Self> {
        let r = Reader::new(bytes, FileKind::Vtx);
        let mut c = r.at(0, "header")?;
        let version = c.read_i32()?;
        if version != VERSION {
            return Err(Error::UnsupportedVersion { file: FileKind::Vtx, version });
        }

        c.skip(12)?; // vertCacheSize, maxBonesPerStrip, maxBonesPerTri, maxBonesPerVert

        let checksum = c.read_u32()?;
        let lod_count = c.read_i32()?;
        if !(1..=MAX_LODS as i32).contains(&lod_count) {
            return Err(r.invalid("LOD count", lod_count, "must be 1 through 8"));
        }

        let replacement_offset = c.read_i32()?;
        let body_part_count = c.read_i32()?;
        let body_part_offset = c.read_i32()?;
        let v49 = mdl_version >= 49;

        let material_replacements = r
            .array(0, replacement_offset, lod_count, REPLACEMENT_LIST_SIZE, "material replacement lists")?
            .map(|at| parse_replacement_list(r, at))
            .collect::<Result<_>>()?;

        let body_parts = r
            .array(0, body_part_offset, body_part_count, BODY_PART_SIZE, "body parts")?
            .map(|at| parse_body_part(r, at, v49))
            .collect::<Result<_>>()?;

        Ok(Self {
            checksum,
            lod_count: lod_count as usize,
            material_replacements,
            body_parts,
        })
    }

    /// The replacement name for `material` at `lod`, if the QC declares one
    pub fn replacement(&self, lod: usize, material: usize) -> Option<&str> {
        self.material_replacements
            .get(lod)?
            .iter()
            .find(|replacement| replacement.material == material)
            .map(|replacement| replacement.name.as_str())
    }
}

fn parse_replacement_list(r: Reader<'_>, at: usize) -> Result<Vec<MaterialReplacement>> {
    let mut c = r.at(at, "material replacement lists")?;
    let count = c.read_i32()?;
    let offset = c.read_i32()?;

    r.array(at, offset, count, REPLACEMENT_SIZE, "material replacements")?
        .map(|at| {
            let mut c = r.at(at, "material replacements")?;
            let material = r.nonnegative(c.read_i16()?.into(), "replacement material")?;
            let name = r.string(at, c.read_i32()?, "material replacements")?;
            Ok(MaterialReplacement { material, name })
        })
        .collect()
}

fn parse_body_part(r: Reader<'_>, at: usize, v49: bool) -> Result<BodyPart> {
    let mut c = r.at(at, "body parts")?;
    let count = c.read_i32()?;
    let offset = c.read_i32()?;

    let models = r
        .array(at, offset, count, MODEL_SIZE, "models")?
        .map(|at| parse_model(r, at, v49))
        .collect::<Result<_>>()?;

    Ok(BodyPart { models })
}

// todo: should i add any useless comments on these func?
fn parse_model(r: Reader<'_>, at: usize, v49: bool) -> Result<Model> {
    let mut c = r.at(at, "models")?;
    let count = c.read_i32()?;
    let offset = c.read_i32()?;

    let lods = r
        .array(at, offset, count, LOD_SIZE, "LODs")?
        .map(|at| parse_lod(r, at, v49))
        .collect::<Result<_>>()?;

    Ok(Model { lods })
}

fn parse_lod(r: Reader<'_>, at: usize, v49: bool) -> Result<Lod> {
    let mut c = r.at(at, "LODs")?;
    let count = c.read_i32()?;
    let offset = c.read_i32()?;
    let switch_distance = c.read_f32()?;

    let meshes = r
        .array(at, offset, count, MESH_SIZE, "meshes")?
        .map(|at| parse_mesh(r, at, v49))
        .collect::<Result<_>>()?;

    Ok(Lod { meshes, switch_distance })
}

fn parse_mesh(r: Reader<'_>, at: usize, v49: bool) -> Result<Mesh> {
    let mut c = r.at(at, "meshes")?;
    let count = c.read_i32()?;
    let offset = c.read_i32()?;
    let flags = MeshFlags::from_bits_retain(c.read_u8()?);
    let size = STRIP_GROUP_SIZE + if v49 { V49_EXTRA_SIZE } else { 0 };

    let strip_groups = r
        .array(at, offset, count, size, "strip groups")?
        .map(|at| parse_strip_group(r, at, v49))
        .collect::<Result<_>>()?;

    Ok(Mesh { strip_groups, flags })
}

fn parse_strip_group(r: Reader<'_>, at: usize, v49: bool) -> Result<StripGroup> {
    let mut c = r.at(at, "strip groups")?;
    let vertex_count = r.nonnegative(c.read_i32()?, "strip group vertex count")?;
    let vertex_offset = c.read_i32()?;
    let index_count = r.nonnegative(c.read_i32()?, "strip group index count")?;
    let index_offset = c.read_i32()?;
    let strip_count = c.read_i32()?;
    let strip_offset = c.read_i32()?;
    let flags = StripGroupFlags::from_bits_retain(c.read_u8()?);

    let mut vertices = Vec::with_capacity(vertex_count);
    for record in r.records::<VERTEX_SIZE>(at, vertex_offset, vertex_count, "strip group vertices")? {
        vertices.push(parse_vertex(r, record)?);
    }

    let mut indices = Vec::with_capacity(index_count);
    for record in r.records::<2>(at, index_offset, index_count, "strip group indices")? {
        let value = u16_at(record, 0);
        index(value, vertices.len(), "strip group vertex")?;
        indices.push(value);
    }

    let size = STRIP_SIZE + if v49 { V49_EXTRA_SIZE } else { 0 };
    let strips = r
        .array(at, strip_offset, strip_count, size, "strips")?
        .map(|at| parse_strip(r, at, indices.len(), vertices.len()))
        .collect::<Result<_>>()?;

    Ok(StripGroup { vertices, indices, strips, flags })
}

fn parse_strip(r: Reader<'_>, at: usize, group_indices: usize, group_vertices: usize) -> Result<Strip> {
    let mut c = r.at(at, "strips")?;
    let strip = Strip {
        index_count: r.nonnegative(c.read_i32()?, "strip index count")?,
        index_offset: r.nonnegative(c.read_i32()?, "strip index offset")?,
        vertex_count: r.nonnegative(c.read_i32()?, "strip vertex count")?,
        vertex_offset: r.nonnegative(c.read_i32()?, "strip vertex offset")?,
        bone_count: c.read_i16()?,
        flags: StripFlags::from_bits_retain(c.read_u8()?),
    };

    r.subrange(strip.index_offset, strip.index_count, group_indices, "strip indices")?;
    r.subrange(strip.vertex_offset, strip.vertex_count, group_vertices, "strip vertices")?;

    Ok(strip)
}

fn parse_vertex(r: Reader<'_>, v: &[u8; VERTEX_SIZE]) -> Result<Vertex> {
    let bone_count = v[3];
    if bone_count > 3 {
        return Err(r.invalid("vertex bone count", bone_count, "a vertex has at most three bones"));
    }

    Ok(Vertex {
        bone_weight_indices: [v[0], v[1], v[2]],
        bone_count,
        original_mesh_vertex: u16_at(v, 4).into(),
        bone_ids: [v[6] as i8, v[7] as i8, v[8] as i8],
    })
}
