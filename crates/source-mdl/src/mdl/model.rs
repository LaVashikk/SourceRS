use crate::cursor::{Reader, i16_at, index};
use crate::vvd::VERTEX_SIZE;
use crate::{MAX_LODS, Result, Vec3};

const MATERIAL_SIZE: usize = 64; // mstudiotexture_t
const BODY_PART_SIZE: usize = 16; // mstudiobodyparts_t
const MODEL_SIZE: usize = 148; // mstudiomodel_t
const MESH_SIZE: usize = 116; // mstudiomesh_t

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Material {
    pub name: String,
    pub flags: u32,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BodyPart {
    pub name: String,
    /// Multiplier of this part's model index in the engine's combined body value
    pub base: i32,
    pub models: Vec<Model>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Model {
    pub name: String,
    pub model_type: i32,
    pub bounding_radius: f32,
    pub vertex_count: usize,
    /// First vertex of this model in the VVD LOD 0 vertex array
    pub vertex_base: usize,
    pub meshes: Vec<Mesh>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Mesh {
    /// Slot in a skin family, see [`super::Mdl::material_index`]
    pub material_slot: usize,
    pub vertex_count: usize,
    /// First vertex relative to the model
    pub vertex_offset: usize,
    pub lod_vertex_counts: [usize; MAX_LODS],
    pub material_type: i32,
    pub material_parameter: i32,
    pub id: i32,
    pub center: Vec3,
}

pub(super) fn parse_materials(r: Reader<'_>, (count, offset): (i32, i32)) -> Result<Vec<Material>> {
    r.array(0, offset, count, MATERIAL_SIZE, "materials")?
        .map(|at| {
            let mut c = r.at(at, "materials")?;
            let name = c.read_i32()?;
            Ok(Material { name: r.string(at, name, "materials")?, flags: c.read_u32()? })
        })
        .collect()
}

pub(super) fn parse_material_directories(r: Reader<'_>, (count, offset): (i32, i32)) -> Result<Vec<String>> {
    r.array(0, offset, count, 4, "material directories")?
        .map(|at| {
            let string = r.at(at, "material directories")?.read_i32()?;
            r.string(0, string, "material directories")
        })
        .collect()
}

pub(super) fn parse_skins(
    r: Reader<'_>,
    (family_count, offset): (i32, i32),
    reference_count: usize,
    material_count: usize,
) -> Result<Vec<Vec<usize>>> {
    // No material slots means zero-sized families: chunks_exact(0) panics,
    // and nothing in the file bounds the family count anymore. So no slots, no skins
    if reference_count == 0 {
        return Ok(Vec::new());
    }

    let family_count = r.nonnegative(family_count, "skin family count")?;
    let slots: Vec<usize> = r
        .records::<2>(0, offset, family_count.saturating_mul(reference_count), "skins")?
        .map(|slot| index(i16_at(slot, 0), material_count, "skin material"))
        .collect::<Result<_>>()?;

    Ok(slots
        .chunks_exact(reference_count)
        .map(<[usize]>::to_vec)
        .collect())
}

pub(super) fn parse_body_parts(
    r: Reader<'_>,
    (count, offset): (i32, i32),
    skin_reference_count: usize,
) -> Result<Vec<BodyPart>> {
    r.array(0, offset, count, BODY_PART_SIZE, "body parts")?
        .map(|at| {
            let mut c = r.at(at, "body parts")?;
            let name = c.read_i32()?;
            let model_count = c.read_i32()?;
            let base = c.read_i32()?;
            let model_offset = c.read_i32()?;
            Ok(BodyPart {
                name: r.string(at, name, "body parts")?,
                base,
                models: r
                    .array(at, model_offset, model_count, MODEL_SIZE, "models")?
                    .map(|at| parse_model(r, at, skin_reference_count))
                    .collect::<Result<_>>()?,
            })
        })
        .collect()
}

fn parse_model(r: Reader<'_>, at: usize, skin_reference_count: usize) -> Result<Model> {
    let name = r.fixed_string(at, 64, "models")?;
    let mut c = r.at(at + 64, "models")?;
    let model_type = c.read_i32()?;
    let bounding_radius = c.read_f32()?;
    let mesh_count = c.read_i32()?;
    let mesh_offset = c.read_i32()?;
    let vertex_count = r.nonnegative(c.read_i32()?, "model vertex count")?;
    let vertex_index = c.read_i32()?;
    let vertex_byte = r.nonnegative(vertex_index, "model vertex index")?;

    if vertex_byte % VERTEX_SIZE != 0 {
        return Err(r.invalid(
            "model vertex index",
            vertex_index,
            "must be a multiple of the 48-byte VVD vertex",
        ));
    }

    Ok(Model {
        name,
        model_type,
        bounding_radius,
        vertex_count,
        vertex_base: vertex_byte / VERTEX_SIZE,
        meshes: r
            .array(at, mesh_offset, mesh_count, MESH_SIZE, "meshes")?
            .map(|at| parse_mesh(r, at, skin_reference_count, vertex_count))
            .collect::<Result<_>>()?,
    })
}

fn parse_mesh(
    r: Reader<'_>,
    at: usize,
    skin_reference_count: usize,
    model_vertex_count: usize,
) -> Result<Mesh> {
    let mut c = r.at(at, "meshes")?;
    let material_slot = index(c.read_i32()?, skin_reference_count, "mesh material slot")?;
    c.skip(4)?; // modelindex

    let vertex_count = r.nonnegative(c.read_i32()?, "mesh vertex count")?;
    let vertex_offset = r.nonnegative(c.read_i32()?, "mesh vertex offset")?;
    c.skip(8)?; // numflexes, flexindex

    let material_type = c.read_i32()?;
    let material_parameter = c.read_i32()?;
    let id = c.read_i32()?;
    let center = c.read_vec3()?;
    c.skip(4)?; // vertexdata.modelvertexdat

    let mut lod_vertex_counts = [0; MAX_LODS];
    for count in &mut lod_vertex_counts {
        *count = r.nonnegative(c.read_i32()?, "mesh LOD vertex count")?;
    }

    if let Some(pair) = lod_vertex_counts
        .windows(2)
        .find(|pair| pair[1] > pair[0])
    {
        return Err(r.invalid(
            "mesh LOD vertex count",
            pair[1] as i64,
            "lower-detail LODs cannot add vertices",
        ));
    }

    // TODO: three shipped Portal 2 models trip this (ballbot_assembly.mdl, vault_door_*.mdl)
    // Find out what it actually trusts here!!
    if lod_vertex_counts[0] != vertex_count {
        return Err(r.invalid(
            "mesh LOD 0 vertex count",
            lod_vertex_counts[0] as i64,
            "must match the mesh vertex count",
        ));
    }
    r.subrange(vertex_offset, vertex_count, model_vertex_count, "mesh vertices")?;

    Ok(Mesh {
        material_slot,
        vertex_count,
        vertex_offset,
        lod_vertex_counts,
        material_type,
        material_parameter,
        id,
        center,
    })
}
