use super::{MaterialReference, StudioModel};
use crate::vtx::{self, StripFlags};
use crate::vvd::BoneWeights;
use crate::{Error, FileKind, Mat3x4, Result, Vec3, Vertex};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GeometryOptions {
    pub lod: usize,
    pub skin: usize,
    /// Model index per body part; missing entries select model 0
    pub bodygroups: Vec<usize>,
    /// One matrix per bone, as produced by [`crate::Pose::skinning`].
    /// Empty keeps the vertices as stored in the VVD (rest pose)
    pub skinning: Vec<Mat3x4>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeometryMesh {
    pub body_part: usize,
    pub model: usize,
    pub mesh: usize,
    /// Triangle list into [`Geometry::vertices`]
    pub indices: Vec<u32>,
    pub material: MaterialReference,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    /// Every vertex of the LOD, including those of unselected bodygroup models
    pub vertices: Vec<Vertex>,
    pub meshes: Vec<GeometryMesh>,
}

impl Geometry {
    /// Flips triangle winding from the file's clockwise D3D9 order to counter-clockwise,
    /// the default front face in wgpu, Vulkan, and OpenGL
    pub fn into_ccw(mut self) -> Self {
        for mesh in &mut self.meshes {
            for triangle in mesh.indices.chunks_exact_mut(3) {
                triangle.swap(1, 2);
            }
        }
        self
    }
}

/// A model picked by the bodygroup selection and its first vertex in the LOD
pub(super) struct Selected {
    pub(super) body_part: usize,
    pub(super) model: usize,
    vertex_base: usize,
}

impl StudioModel {
    /// Extracts an owned triangle-list representation without changing axes, UVs, handedness, or triangle winding
    pub fn geometry(&self, options: &GeometryOptions) -> Result<Geometry> {
        let selection = self.selection(options)?;
        let lod = options.lod;

        let mut vertices = self.vvd.lod_vertices(lod)?;
        if !options.skinning.is_empty() {
            // TODO: tangents stay in the rest pose, so normal maps on a posed model are subtly fucked
            // i hate all of this... why im still here
            for vertex in &mut vertices {
                vertex.position = blend(vertex.position, &vertex.bone_weights, &options.skinning, true);
                vertex.normal = blend(vertex.normal, &vertex.bone_weights, &options.skinning, false);
            }
        }

        let mut meshes = Vec::new();
        for Selected { body_part, model, vertex_base } in selection {
            let mdl_model = &self.mdl.body_parts[body_part].models[model];
            let vtx_lod = &self.vtx.body_parts[body_part].models[model].lods[lod];
            let mut base = vertex_base;
            for (mesh, (mdl_mesh, vtx_mesh)) in mdl_model
                .meshes
                .iter()
                .zip(&vtx_lod.meshes)
                .enumerate()
            {
                let vertex_count = mdl_mesh.lod_vertex_counts[lod];
                meshes.push(GeometryMesh {
                    body_part,
                    model,
                    mesh,
                    indices: mesh_indices(vtx_mesh, base, vertex_count)?,
                    material: self.material_reference(lod, options.skin, mdl_mesh.material_slot)?,
                });
                base += vertex_count;
            }
        }

        Ok(Geometry { vertices, meshes })
    }

    pub(super) fn selection(&self, options: &GeometryOptions) -> Result<Vec<Selected>> {
        let lod = options.lod;
        if lod >= self.vtx.lod_count {
            return Err(Error::index("LOD", lod, self.vtx.lod_count));
        }
        let mut selection = Vec::new();

        // TODO: this layout is our reading of Studio_SetRootLOD
        // Never checked against the engine for non-default bodygroups. Needs a fixture: 2 body parts x 2 models x 2 LODs + fixups
        let mut vertex_base = 0;
        for (body_part, body) in self.mdl.body_parts.iter().enumerate() {
            let pick = options
                .bodygroups
                .get(body_part)
                .copied()
                .unwrap_or(0);

            if pick >= body.models.len() {
                return Err(Error::index("bodygroup model", pick, body.models.len()));
            }

            for (model, mdl_model) in body.models.iter().enumerate() {
                if model == pick {
                    selection.push(Selected { body_part, model, vertex_base });
                }
                vertex_base += mdl_model
                    .meshes
                    .iter()
                    .map(|mesh| mesh.lod_vertex_counts[lod])
                    .sum::<usize>();
            }
        }
        Ok(selection)
    }
}

/// Triangle-list indices of one mesh, offset by its first vertex in the LOD
fn mesh_indices(mesh: &vtx::Mesh, base: usize, vertex_count: usize) -> Result<Vec<u32>> {
    let mut indices = Vec::new();
    // todo:
    for group in &mesh.strip_groups {
        for strip in &group.strips {
            // todo: optimize it someday
            let resolved = group.indices[strip.index_offset..][..strip.index_count]
                .iter()
                .map(|&index| {
                    let vertex = group.vertices[usize::from(index)].original_mesh_vertex;
                    if vertex >= vertex_count {
                        return Err(Error::index("VTX original mesh vertex", vertex, vertex_count));
                    }
                    Ok((base + vertex) as u32)
                })
                .collect::<Result<Vec<_>>>()?;
            triangulate(strip.flags, &resolved, &mut indices)?;
        }
    }

    Ok(indices)
}

fn triangulate(flags: StripFlags, source: &[u32], output: &mut Vec<u32>) -> Result<()> {
    match (flags.contains(StripFlags::TRIANGLE_LIST), flags.contains(StripFlags::TRIANGLE_STRIP)) {
        (true, false) => {
            if source.len() % 3 != 0 {
                return Err(Error::InvalidField {
                    file: FileKind::Vtx,
                    field: "triangle-list index count",
                    value: source.len() as i64,
                    reason: "triangle lists require a multiple of three indices",
                });
            }
            output.extend_from_slice(source);
        }

        (false, true) => {
            for index in 2..source.len() {
                let triangle = if index % 2 == 0 {
                    [source[index - 2], source[index - 1], source[index]]
                } else {
                    [source[index - 1], source[index - 2], source[index]]
                };
                if triangle[0] != triangle[1] && triangle[1] != triangle[2] && triangle[0] != triangle[2] {
                    output.extend_from_slice(&triangle);
                }
            }
        }

        // TODO: urban_puddle_model02a.mdl has a strip with flags 0x00 and lands here
        // Pretty sure the engine draws anything that isn't a strip as a list, check it and stop being picky
        _ => {
            return Err(Error::InvalidField {
                file: FileKind::Vtx,
                field: "strip flags",
                value: flags.bits().into(),
                reason: "a strip must be either a triangle list or a triangle strip",
            });
        }
    }
    Ok(())
}

/// Weighted sum of the bone matrices applied to one vertex, `translate` is off for directions
///
/// A vertex with no weights is bound to bone 0, as studiomdl writes models collapsed to a single bone
fn blend(value: Vec3, weights: &BoneWeights, matrices: &[Mat3x4], translate: bool) -> Vec3 {
    let used = usize::from(weights.count).clamp(1, weights.bones.len());
    let mut out = [0.0f32; 3];
    let mut total = 0.0f32;

    for slot in 0..used {
        let Some(matrix) = matrices.get(usize::from(weights.bones[slot])) else {
            continue;
        };
        let weight = if weights.count == 0 { 1.0 } else { weights.weights[slot] };
        for axis in 0..3 {
            let row = matrix[axis];
            out[axis] += weight
                * (row[0] * value[0]
                    + row[1] * value[1]
                    + row[2] * value[2]
                    + if translate { row[3] } else { 0.0 });
        }
        total += weight;
    }

    if total <= f32::EPSILON {
        return value;
    }

    [out[0] / total, out[1] / total, out[2] / total]
}
