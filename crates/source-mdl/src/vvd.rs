//! Vertex data: `vertexFileHeader_t` and the records it links, from the SDK's `studio.h`

use crate::cursor::{Reader, f32_at, index, vec3_at};
use crate::{Error, FileKind, MAX_LODS, Result, Vec2, Vec3, Vec4};

const SIGNATURE: u32 = u32::from_le_bytes(*b"IDSV");
const VERSION: i32 = 4;
const FIXUP_SIZE: usize = 12; // vertexFileFixup_t
pub(crate) const VERTEX_SIZE: usize = 48; // mstudiovertex_t
const TANGENT_SIZE: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BoneWeights {
    pub weights: [f32; 3],
    pub bones: [u8; 3],
    pub count: u8,
}

// TODO Portal 2's a4_destruction/laserplat_dest_b.mdl ships 8052 NaN floats in its vert-data, wtf Valve :/
// fix and then fix it again and then fix it again... someday... prob never
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vertex {
    pub bone_weights: BoneWeights,
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub tangent: Option<Vec4>,
}

/// A run of LOD 0 vertices that is used by every LOD up to and including `lod`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Fixup {
    pub lod: usize,
    pub source_vertex: usize,
    pub vertex_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vvd {
    pub checksum: u32,
    pub lod_count: usize,
    /// Only the first `lod_count` entries are meaningful
    pub lod_vertex_counts: [usize; MAX_LODS],
    pub fixups: Vec<Fixup>,
    /// LOD 0 vertices in file order
    pub vertices: Vec<Vertex>,
}

impl Vvd {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let r = Reader::new(bytes, FileKind::Vvd);
        let mut c = r.at(0, "header")?;
        let signature = c.read_u32()?;
        if signature != SIGNATURE {
            return Err(Error::InvalidSignature {
                file: FileKind::Vvd,
                expected: SIGNATURE,
                actual: signature,
            });
        }

        let version = c.read_i32()?;
        if version != VERSION {
            return Err(Error::UnsupportedVersion { file: FileKind::Vvd, version });
        }

        let checksum = c.read_u32()?;
        let lod_count = c.read_i32()?;
        if !(1..=MAX_LODS as i32).contains(&lod_count) {
            return Err(r.invalid("LOD count", lod_count, "must be 1 through 8"));
        }
        let lod_count = lod_count as usize;
        let mut lod_vertex_counts = [0; MAX_LODS];
        for count in &mut lod_vertex_counts {
            *count = r.nonnegative(c.read_i32()?, "LOD vertex count")?;
        }

        if let Some(pair) = lod_vertex_counts[..lod_count]
            .windows(2)
            .find(|pair| pair[1] > pair[0])
        {
            return Err(r.invalid(
                "LOD vertex count",
                pair[1] as i64,
                "lower-detail LODs cannot add vertices",
            ));
        }

        let fixup_count = c.read_i32()?;
        let fixup_offset = c.read_i32()?;
        let vertex_offset = c.read_i32()?;
        let tangent_offset = c.read_i32()?;

        let vertex_count = lod_vertex_counts[0];
        let fixups = r
            .array(0, fixup_offset, fixup_count, FIXUP_SIZE, "fixups")?
            .map(|at| parse_fixup(r, at, lod_count, vertex_count))
            .collect::<Result<Vec<_>>>()?;

        for (lod, &expected) in lod_vertex_counts[..lod_count].iter().enumerate() {
            check_fixups(&fixups, lod, expected)?;
        }

        let mut vertices = Vec::with_capacity(vertex_count);
        for record in r.records::<VERTEX_SIZE>(0, vertex_offset, vertex_count, "vertices")? {
            let vertex = parse_vertex(record);
            if vertex.bone_weights.count > 3 {
                return Err(r.invalid(
                    "vertex bone count",
                    vertex.bone_weights.count,
                    "a vertex has at most three bones",
                ));
            }
            vertices.push(vertex);
        }

        if tangent_offset != 0 {
            let tangents = r.records::<TANGENT_SIZE>(0, tangent_offset, vertex_count, "tangents")?;
            for (vertex, t) in vertices.iter_mut().zip(tangents) {
                vertex.tangent = Some([f32_at(t, 0), f32_at(t, 4), f32_at(t, 8), f32_at(t, 12)]);
            }
        }

        Ok(Self { checksum, lod_count, lod_vertex_counts, fixups, vertices })
    }

    /// Vertices of root LOD `lod` in mesh order, with fixups applied
    pub fn lod_vertices(&self, lod: usize) -> Result<Vec<Vertex>> {
        if lod >= self.lod_count {
            return Err(Error::index("VVD LOD", lod, self.lod_count));
        }
        if self.fixups.is_empty() {
            return Ok(self.vertices[..self.lod_vertex_counts[lod]].to_vec());
        }

        Ok(self
            .fixups
            .iter()
            .filter(|fixup| fixup.lod >= lod)
            .flat_map(|fixup| &self.vertices[fixup.source_vertex..][..fixup.vertex_count])
            .copied()
            .collect())
    }
}

fn parse_fixup(r: Reader<'_>, at: usize, lod_count: usize, vertex_count: usize) -> Result<Fixup> {
    let mut c = r.at(at, "fixups")?;
    let lod = index(c.read_i32()?, lod_count, "fixup LOD")?;
    let source_vertex = r.nonnegative(c.read_i32()?, "fixup source vertex")?;
    let count = r.nonnegative(c.read_i32()?, "fixup vertex count")?;

    r.subrange(source_vertex, count, vertex_count, "fixup vertices")?;

    Ok(Fixup { lod, source_vertex, vertex_count: count })
}

fn check_fixups(fixups: &[Fixup], lod: usize, expected: usize) -> Result<()> {
    if fixups.is_empty() {
        return Ok(());
    }

    let actual = fixups
        .iter()
        .filter(|fixup| fixup.lod >= lod)
        .fold(0usize, |sum, fixup| sum.saturating_add(fixup.vertex_count));

    if actual != expected {
        return Err(Error::StructureMismatch { context: "VVD fixup vertex count", expected, actual });
    }

    Ok(())
}

fn parse_vertex(v: &[u8; VERTEX_SIZE]) -> Vertex {
    Vertex {
        bone_weights: BoneWeights { weights: vec3_at(v, 0), bones: [v[12], v[13], v[14]], count: v[15] },
        position: vec3_at(v, 16),
        normal: vec3_at(v, 28),
        uv: [f32_at(v, 40), f32_at(v, 44)],
        tangent: None,
    }
}
