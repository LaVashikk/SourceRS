use super::{Animation, PositionTrack, RotationTrack};
use crate::{Error, FileKind, Mat3x4, Mdl, Quat, Result, Vec3};

/// Where every bone sits, in model space
#[derive(Debug, Clone, PartialEq)]
pub struct Pose {
    /// One matrix per bone
    pub world: Vec<Mat3x4>,
}

impl Pose {
    /// The skeleton as authored, with no animation applied
    pub fn rest(mdl: &Mdl) -> Self {
        Self { world: build(mdl, |_| (None, None)) }
    }

    /// One frame of a sequence. Only the first animation of its blend grid is sampled
    pub fn from_sequence(mdl: &Mdl, sequence: usize, frame: usize) -> Result<Self> {
        // todo: delta sequences (STUDIO_DELTA, 0x4) get built as an absolute pose here, which is garbage
        // Either reject them or stack them on top of the rest pose
        let sequence =
            mdl.sequences
                .get(sequence)
                .ok_or(Error::index("sequence", sequence, mdl.sequences.len()))?;

        // todo for now: no blending, we just grab the first animation of the grid. idk slop this part and be happy
        let Some(&index) = sequence.animations.first() else {
            return Ok(Self::rest(mdl));
        };

        let animation = mdl.animations.get(index).ok_or(Error::index(
            "sequence animation",
            index,
            mdl.animations.len(),
        ))?;

        if let Some(feature) = animation.unsupported {
            return Err(Error::UnsupportedFeature { file: FileKind::Mdl, feature });
        }

        if frame >= animation.frame_count {
            return Err(Error::index("animation frame", frame, animation.frame_count));
        }

        Ok(Self { world: build(mdl, |bone| sample(mdl, animation, bone, frame)) })
    }

    /// Per-bone `world * poseToBone` matrices for [`crate::GeometryOptions::skinning`]
    /// VVD vertices are in model space, so at rest these are identities
    pub fn skinning(&self, mdl: &Mdl) -> Vec<Mat3x4> {
        mdl.bones
            .iter()
            .enumerate()
            .map(|(index, bone)| match self.world.get(index) {
                Some(world) => concat(*world, bone.pose_to_bone),
                None => IDENTITY,
            })
            .collect()
    }
}

const IDENTITY: Mat3x4 = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]];

/// Single pass in file order; studiomdl writes parents before their children
fn build(mdl: &Mdl, mut override_for: impl FnMut(usize) -> (Option<Quat>, Option<Vec3>)) -> Vec<Mat3x4> {
    let mut world: Vec<Mat3x4> = Vec::with_capacity(mdl.bones.len());

    for (index, bone) in mdl.bones.iter().enumerate() {
        let (rotation, position) = override_for(index);
        let local = compose(rotation.unwrap_or(bone.rotation_quat), position.unwrap_or(bone.position));
        let matrix = match bone.parent {
            Some(parent) => match world.get(parent) {
                Some(parent) => concat(*parent, local),
                // Parent after its child? studiomdl never writes that, but if some tool does, the bone just becomes a root
                None => local,
            },
            None => local,
        };
        world.push(matrix);
    }
    world
}

/// Rotation and position of one bone at `frame`; `None` keeps the rest value
fn sample(mdl: &Mdl, animation: &Animation, bone: usize, frame: usize) -> (Option<Quat>, Option<Vec3>) {
    // todo: this code smells like shit.. fix it
    let Some(track) = animation
        .tracks
        .iter()
        .find(|track| track.bone == bone)
    else {
        return (None, None);
    };

    let Some(rest) = mdl.bones.get(bone) else {
        return (None, None);
    };

    let rotation = match &track.rotation {
        RotationTrack::Rest => None,
        // Careful: a constant quaternion is absolute even on a delta track, only animated values are deltas
        RotationTrack::Constant(quat) => Some(*quat),
        RotationTrack::Animated(streams) => {
            let scale = rest.rotation_scale;
            let base = if track.delta { [0.0; 3] } else { rest.rotation_euler };
            Some(euler_to_quat([
                base[0] + streams[0].at(frame) * scale[0],
                base[1] + streams[1].at(frame) * scale[1],
                base[2] + streams[2].at(frame) * scale[2],
            ]))
        }
    };

    let position = match &track.position {
        PositionTrack::Rest => None,
        PositionTrack::Constant(vec) => Some(*vec),
        PositionTrack::Animated(streams) => {
            let scale = rest.position_scale;
            let base = if track.delta { [0.0; 3] } else { rest.position };
            Some([
                base[0] + streams[0].at(frame) * scale[0],
                base[1] + streams[1].at(frame) * scale[1],
                base[2] + streams[2].at(frame) * scale[2],
            ])
        }
    };

    (rotation, position)
}

fn compose(quat: Quat, position: Vec3) -> Mat3x4 {
    let [x, y, z, w] = quat;
    [
        [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y - z * w), 2.0 * (x * z + y * w), position[0]],
        [2.0 * (x * y + z * w), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z - x * w), position[1]],
        [2.0 * (x * z - y * w), 2.0 * (y * z + x * w), 1.0 - 2.0 * (x * x + y * y), position[2]],
    ]
}

fn concat(outer: Mat3x4, inner: Mat3x4) -> Mat3x4 {
    let mut out = [[0.0f32; 4]; 3];
    for row in 0..3 {
        for column in 0..3 {
            out[row][column] = (0..3)
                .map(|k| outer[row][k] * inner[k][column])
                .sum();
        }
        out[row][3] = (0..3)
            .map(|k| outer[row][k] * inner[k][3])
            .sum::<f32>()
            + outer[row][3];
    }
    out
}

/// Source's `AngleQuaternion` for a `RadianEuler`: roll about X, then pitch, then yaw
fn euler_to_quat(angles: Vec3) -> Quat {
    let (sr, cr) = (angles[0] * 0.5).sin_cos();
    let (sp, cp) = (angles[1] * 0.5).sin_cos();
    let (sy, cy) = (angles[2] * 0.5).sin_cos();
    [
        sr * cp * cy - cr * sp * sy,
        cr * sp * cy + sr * cp * sy,
        cr * cp * sy - sr * sp * cy,
        cr * cp * cy + sr * sp * sy,
    ]
}
