//! Sequences, the animation tracks they play, and bone poses built from them
//!
//! A sequence references animations through a blend grid.
//! An animation holds per-bone tracks, bones without a track keep their rest transform.
//! A track stores rotation and position either as a constant or as a run-length encoded stream per axis

mod parse;
mod pose;

pub(crate) use parse::{parse_animations, parse_sequences};
pub use pose::Pose;

use crate::{Quat, Vec3};

/// A named entry in the sequence table
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sequence {
    pub name: String,
    pub flags: i32,
    /// Indices into [`crate::Mdl::animations`], laid out as `blend_size[0] * blend_size[1]`
    pub animations: Vec<usize>,
    pub blend_size: [usize; 2],
}

/// One animation track set: what every bone does over `frame_count` frames
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Animation {
    pub name: String,
    pub fps: f32,
    pub frame_count: usize,
    /// Tracks for the bones this animation moves; other bones stay at rest
    pub tracks: Vec<BoneTrack>,
    /// Why the tracks were not read (external `.ani` block, sectioned or malformed data).
    /// `tracks` is empty when this is set
    pub unsupported: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BoneTrack {
    pub bone: usize,
    /// Values are a correction to the bone's rest transform rather than a replacement
    pub delta: bool,
    pub rotation: RotationTrack,
    pub position: PositionTrack,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RotationTrack {
    Rest,
    Constant(Quat),
    /// Euler radians per axis, to be scaled by the bone's `rotation_scale`
    Animated([ValueStream; 3]),
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PositionTrack {
    Rest,
    Constant(Vec3),
    /// Units per axis, to be scaled by the bone's `position_scale`
    Animated([ValueStream; 3]),
}

/// Run-length encoded values, one axis of one bone
///
/// The stream is a chain of blocks.
/// A block starts with a `(valid, total)` byte pair: `valid` explicit values follow and cover `total` frames, the last value repeating to fill the gap.
/// Blocks continue until the animation's frames are covered
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValueStream {
    pub words: Vec<i16>,
}

impl ValueStream {
    /// The value at `frame`, before the bone's scale is applied
    pub fn at(&self, frame: usize) -> f32 {
        let mut remaining = frame;
        let mut cursor = 0usize;

        while let Some(&header) = self.words.get(cursor) {
            let valid = usize::from(header.to_le_bytes()[0]);
            let total = usize::from(header.to_le_bytes()[1]);
            if total == 0 {
                break;
            }

            if remaining < total {
                // Out of explicit values: the last one just keeps repeating till the block ends
                let index = cursor + 1 + remaining.min(valid.saturating_sub(1));
                return self.words.get(index).copied().unwrap_or(0).into();
            }

            remaining -= total;
            cursor += valid + 1;
        }
        0.0
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}
