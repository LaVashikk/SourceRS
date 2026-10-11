use bitflags::bitflags;

use crate::cursor::{Reader, index};
use crate::{Mat3x4, Quat, Result, Vec3};

const BONE_SIZE: usize = 216; // mstudiobone_t
const BONE_CONTROLLER_SIZE: usize = 56; // mstudiobonecontroller_t
const HITBOX_SET_SIZE: usize = 12; // mstudiohitboxset_t
const HITBOX_SIZE: usize = 68; // mstudiobbox_t

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct BoneFlags: u32 {
        const PHYSICALLY_SIMULATED = 1 << 0;
        const PHYSICS_PROCEDURAL = 1 << 1;
        const ALWAYS_PROCEDURAL = 1 << 2;
        const SCREEN_ALIGN_SPHERE = 1 << 3;
        const SCREEN_ALIGN_CYLINDER = 1 << 4;
        const USED_BY_HITBOX = 1 << 8;
        const USED_BY_ATTACHMENT = 1 << 9;
        const USED_BY_VERTEX_LOD0 = 1 << 10;
        const USED_BY_VERTEX_LOD1 = 1 << 11;
        const USED_BY_VERTEX_LOD2 = 1 << 12;
        const USED_BY_VERTEX_LOD3 = 1 << 13;
        const USED_BY_VERTEX_LOD4 = 1 << 14;
        const USED_BY_VERTEX_LOD5 = 1 << 15;
        const USED_BY_VERTEX_LOD6 = 1 << 16;
        const USED_BY_VERTEX_LOD7 = 1 << 17;
        const USED_BY_BONE_MERGE = 1 << 18;
        const FIXED_ALIGNMENT = 1 << 20;
        const HAS_SAVEFRAME_POS = 1 << 21;
        const HAS_SAVEFRAME_ROT = 1 << 22;
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bone {
    pub name: String,
    pub parent: Option<usize>,
    /// Controller index per channel (position X/Y/Z, rotation X/Y/Z), `-1` for none
    pub bone_controllers: [i32; 6],
    pub position: Vec3,
    pub rotation_quat: Quat,
    pub rotation_euler: Vec3,
    pub position_scale: Vec3,
    pub rotation_scale: Vec3,
    pub pose_to_bone: Mat3x4,
    pub alignment: Quat,
    pub flags: BoneFlags,
    pub procedure_type: i32,
    pub procedure_offset: i32,
    pub physics_bone: i32,
    pub surface_property: String,
    pub contents: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BoneController {
    /// `-1` for none
    pub bone: i32,
    pub controller_type: i32,
    pub start: f32,
    pub end: f32,
    pub rest: i32,
    pub input_field: i32,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HitboxSet {
    pub name: String,
    pub hitboxes: Vec<Hitbox>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Hitbox {
    pub bone: usize,
    pub group: i32,
    pub min: Vec3,
    pub max: Vec3,
    pub name: String,
}

pub(super) fn parse_bones(
    r: Reader<'_>,
    (count, offset): (i32, i32),
    controller_count: usize,
) -> Result<Vec<Bone>> {
    let records = r.array(0, offset, count, BONE_SIZE, "bones")?;
    let bone_count = records.len();
    records
        .map(|at| parse_bone(r, at, bone_count, controller_count))
        .collect()
}

fn parse_bone(r: Reader<'_>, at: usize, bone_count: usize, controller_count: usize) -> Result<Bone> {
    let mut c = r.at(at, "bones")?;
    let name = c.read_i32()?;
    let parent = c.read_i32()?;
    let mut bone_controllers = [-1; 6];
    for controller in &mut bone_controllers {
        *controller = c.read_i32()?;
        if *controller != -1 {
            index(*controller, controller_count, "bone controller")?;
        }
    }
    Ok(Bone {
        name: r.string(at, name, "bones")?,
        parent: match parent {
            -1 => None,
            parent => Some(index(parent, bone_count, "bone parent")?),
        },
        bone_controllers,
        position: c.read_vec3()?,
        rotation_quat: c.read_vec4()?,
        rotation_euler: c.read_vec3()?,
        position_scale: c.read_vec3()?,
        rotation_scale: c.read_vec3()?,
        pose_to_bone: c.read_mat3x4()?,
        alignment: c.read_vec4()?,
        flags: BoneFlags::from_bits_retain(c.read_u32()?),
        procedure_type: c.read_i32()?,
        procedure_offset: c.read_i32()?,
        physics_bone: c.read_i32()?,
        surface_property: r.string(at, c.read_i32()?, "bones")?,
        contents: c.read_i32()?,
    })
}

pub(super) fn parse_bone_controllers(
    r: Reader<'_>,
    (count, offset): (i32, i32),
    bone_count: usize,
) -> Result<Vec<BoneController>> {
    r.array(0, offset, count, BONE_CONTROLLER_SIZE, "bone controllers")?
        .map(|at| {
            let mut c = r.at(at, "bone controllers")?;
            let controller = BoneController {
                bone: c.read_i32()?,
                controller_type: c.read_i32()?,
                start: c.read_f32()?,
                end: c.read_f32()?,
                rest: c.read_i32()?,
                input_field: c.read_i32()?,
            };
            if controller.bone != -1 {
                index(controller.bone, bone_count, "bone controller bone")?;
            }
            Ok(controller)
        })
        .collect()
}

pub(super) fn parse_hitbox_sets(
    r: Reader<'_>,
    (count, offset): (i32, i32),
    bone_count: usize,
) -> Result<Vec<HitboxSet>> {
    r.array(0, offset, count, HITBOX_SET_SIZE, "hitbox sets")?
        .map(|at| parse_hitbox_set(r, at, bone_count))
        .collect()
}

// Some release models point hitbox (and hitbox set) names past EOF
// A hitbox without a name is still a perfectly fine hitbox, so a broken name just becomes ""
fn parse_hitbox_set(r: Reader<'_>, at: usize, bone_count: usize) -> Result<HitboxSet> {
    let mut c = r.at(at, "hitbox sets")?;
    let name = c.read_i32()?;
    let count = c.read_i32()?;
    let offset = c.read_i32()?;
    Ok(HitboxSet {
        name: r
            .string(at, name, "hitbox sets")
            .unwrap_or_default(),
        hitboxes: r
            .array(at, offset, count, HITBOX_SIZE, "hitboxes")?
            .map(|at| parse_hitbox(r, at, bone_count))
            .collect::<Result<_>>()?,
    })
}

fn parse_hitbox(r: Reader<'_>, at: usize, bone_count: usize) -> Result<Hitbox> {
    let mut c = r.at(at, "hitboxes")?;
    Ok(Hitbox {
        bone: index(c.read_i32()?, bone_count, "hitbox bone")?,
        group: c.read_i32()?,
        min: c.read_vec3()?,
        max: c.read_vec3()?,
        name: r
            .string(at, c.read_i32()?, "hitboxes")
            .unwrap_or_default(),
    })
}
