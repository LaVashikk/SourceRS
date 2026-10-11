use super::{Animation, BoneTrack, PositionTrack, RotationTrack, Sequence, ValueStream};
use crate::cursor::{Reader, i16_at, index};
use crate::{Quat, Result};

const ANIMATION_SIZE: usize = 100; // mstudioanimdesc_t
const SEQUENCE_SIZE: usize = 212; // mstudioseqdesc_t

const RAW_POSITION: u8 = 0x01;
const RAW_ROTATION: u8 = 0x02;
const ANIMATED_POSITION: u8 = 0x04;
const ANIMATED_ROTATION: u8 = 0x08;
const DELTA: u8 = 0x10;
const RAW_ROTATION_64: u8 = 0x20;

const TRACK_HEADER_SIZE: usize = 4;
const VALUE_POINTER_SIZE: usize = 6;
const QUAT48_SIZE: usize = 6;
const QUAT64_SIZE: usize = 8;

pub(crate) fn parse_sequences(
    r: Reader<'_>,
    (count, offset): (i32, i32),
    animation_count: usize,
) -> Result<Vec<Sequence>> {
    r.array(0, offset, count, SEQUENCE_SIZE, "sequences")?
        .map(|at| parse_sequence(r, at, animation_count))
        .collect()
}

fn parse_sequence(r: Reader<'_>, at: usize, animation_count: usize) -> Result<Sequence> {
    let mut c = r.at(at, "sequences")?;
    c.skip(4)?; // baseptr

    let name = c.read_i32()?;
    c.skip(4)?; // szactivitynameindex

    let flags = c.read_i32()?;
    c.skip(40)?; // activity, actweight, numevents, eventindex, bbmin, bbmax

    let blend_count = c.read_i32()?;
    let blend_offset = c.read_i32()?;
    c.skip(4)?; // movementindex

    let blend_size = [
        r.nonnegative(c.read_i32()?, "sequence blend width")?,
        r.nonnegative(c.read_i32()?, "sequence blend height")?,
    ];

    // todo todo no idea which one the engine trusts, so the bigger one wins
    let declared = blend_size[0]
        .saturating_mul(blend_size[1])
        .max(blend_count.max(0) as usize);

    let animations = r
        .records::<2>(at, blend_offset, declared, "sequence blend table")?
        .map(|entry| {
            // TODO: smokestack.mdl has a blend entry past the animation count and dies right here, taking its geometry with it
            // Same deal as broken tracks: this should cost the sequence, NOT the whole model
            index(i16_at(entry, 0), animation_count, "sequence blend entry")
        })
        .collect::<Result<_>>()?;

    Ok(Sequence { name: r.string(at, name, "sequences")?, flags, animations, blend_size })
}

pub(crate) fn parse_animations(
    r: Reader<'_>,
    (count, offset): (i32, i32),
    bone_count: usize,
) -> Result<Vec<Animation>> {
    r.array(0, offset, count, ANIMATION_SIZE, "animations")?
        .map(|at| parse_animation(r, at, bone_count))
        .collect()
}

fn parse_animation(r: Reader<'_>, at: usize, bone_count: usize) -> Result<Animation> {
    let mut c = r.at(at, "animations")?;
    c.skip(4)?; // baseptr

    let name = c.read_i32()?;
    let fps = c.read_f32()?;
    c.skip(4)?; // flags

    let frame_count = r.nonnegative(c.read_i32()?, "animation frame count")?;
    c.skip(32)?; // nummovements, movementindex, unused1[6]

    let block = c.read_i32()?;
    let track_offset = c.read_i32()?;
    c.skip(20)?; // numikrules, ikruleindex, animblockikruleindex, numlocalhierarchy, localhierarchyindex

    let section_index = c.read_i32()?;

    let (tracks, unsupported) = if block != 0 {
        (Vec::new(), Some("animation data stored in an external .ani block"))
    } else if section_index != 0 {
        (Vec::new(), Some("sectioned animation data"))
    } else {
        match parse_tracks(r, at, track_offset, frame_count, bone_count) {
            Ok(tracks) => (tracks, None),
            // todo: bruh, every single animation of ballbot_animations.mdl (212/212) and container_ride/* lands here
            // Track bones point way past the bone count, so we're misreading some Portal 2 layout prob?
            // or this one broken from beta?
            Err(_) => (Vec::new(), Some("malformed animation tracks")),
        }
    };

    Ok(Animation {
        name: r.string(at, name, "animations")?,
        fps,
        frame_count,
        tracks,
        unsupported,
    })
}

fn parse_tracks(
    r: Reader<'_>,
    base: usize,
    relative: i32,
    frame_count: usize,
    bone_count: usize,
) -> Result<Vec<BoneTrack>> {
    if relative == 0 || bone_count == 0 {
        return Ok(Vec::new());
    }
    let mut position = r.offset(base, relative, "animation tracks")?;
    let mut tracks = Vec::new();

    // Tracks are a linked list of relative `next` offsets, 0 ends it
    // One track per bone max, otherwise a corrupt `next` loops us forever
    for _ in 0..bone_count {
        let mut c = r.at(position, "animation tracks")?;
        let bone_raw = c.read_u8()?;
        // studiomdl's homemade "end of list" for models collapsed to a single bone
        if bone_raw == 0xff {
            break;
        }
        let flags = c.read_u8()?;
        let next = c.read_i16()?;
        let bone = index(bone_raw, bone_count, "animation track bone")?;

        let data = position + TRACK_HEADER_SIZE;
        tracks.push(BoneTrack {
            bone,
            delta: flags & DELTA != 0,
            rotation: read_rotation(r, data, flags, frame_count)?,
            position: read_position(r, data, flags, frame_count)?,
        });

        if next == 0 {
            break;
        }

        position = r.offset(position, i32::from(next), "animation track chain")?;
    }
    Ok(tracks)
}

fn read_rotation(r: Reader<'_>, data: usize, flags: u8, frame_count: usize) -> Result<RotationTrack> {
    if flags & ANIMATED_ROTATION != 0 {
        return Ok(RotationTrack::Animated(read_streams(r, data, frame_count)?));
    }

    if flags & RAW_ROTATION != 0 {
        let mut c = r.at(data, "animation tracks")?;
        return Ok(RotationTrack::Constant(quat48(&[c.read_u16()?, c.read_u16()?, c.read_u16()?])));
    }

    if flags & RAW_ROTATION_64 != 0 {
        let mut c = r.at(data, "animation tracks")?;
        let low = c.read_u32()?;
        let high = c.read_u32()?;
        return Ok(RotationTrack::Constant(quat64(u64::from(low) | (u64::from(high) << 32))));
    }

    Ok(RotationTrack::Rest)
}

fn read_position(r: Reader<'_>, data: usize, flags: u8, frame_count: usize) -> Result<PositionTrack> {
    if flags & ANIMATED_POSITION != 0 {
        // Both animated: the rotation's value pointers come first, position ones after them
        let offset = data + usize::from(flags & ANIMATED_ROTATION != 0) * VALUE_POINTER_SIZE;
        return Ok(PositionTrack::Animated(read_streams(r, offset, frame_count)?));
    }

    if flags & RAW_POSITION != 0 {
        // Vector48 sits right after the constant rotation, if there is one
        let offset = data
            + usize::from(flags & RAW_ROTATION != 0) * QUAT48_SIZE
            + usize::from(flags & RAW_ROTATION_64 != 0) * QUAT64_SIZE;
        let mut c = r.at(offset, "animation tracks")?;
        return Ok(PositionTrack::Constant([f16(c.read_u16()?), f16(c.read_u16()?), f16(c.read_u16()?)]));
    }

    Ok(PositionTrack::Rest)
}

/// Three `mstudioanimvalue_t` streams, addressed by offsets relative to the pointer block
fn read_streams(r: Reader<'_>, base: usize, frame_count: usize) -> Result<[ValueStream; 3]> {
    let mut c = r.at(base, "animation values")?;
    let offsets = [c.read_i16()?, c.read_i16()?, c.read_i16()?];

    let mut streams = [ValueStream::default(), ValueStream::default(), ValueStream::default()];
    for (axis, relative) in offsets.into_iter().enumerate() {
        if relative <= 0 {
            continue;
        }
        let start = r.offset(base, i32::from(relative), "animation values")?;
        streams[axis] = read_stream(r, start, frame_count)?;
    }

    Ok(streams)
}

fn read_stream(r: Reader<'_>, start: usize, frame_count: usize) -> Result<ValueStream> {
    let mut c = r.at(start, "animation values")?;
    let mut words = Vec::new();
    let mut covered = 0usize;

    while covered < frame_count {
        let header = c.read_i16()?;
        let valid = usize::from(header.to_le_bytes()[0]);
        let total = usize::from(header.to_le_bytes()[1]);
        if total == 0 {
            return Err(r.invalid(
                "animation value block",
                header,
                "a block has to cover at least one frame",
            ));
        }

        words.push(header);
        for _ in 0..valid {
            words.push(c.read_i16()?);
        }
        covered += total;
    }
    Ok(ValueStream { words })
}

/// 16 bits per axis, `w` rebuilt from the norm and the sign bit packed into `z`
fn quat48(raw: &[u16; 3]) -> Quat {
    let x = (f32::from(raw[0]) - 32768.0) / 32768.0;
    let y = (f32::from(raw[1]) - 32768.0) / 32768.0;
    let z = (f32::from(raw[2] & 0x7fff) - 16384.0) / 16384.0;
    let w = (1.0 - x * x - y * y - z * z).max(0.0).sqrt();
    let w = if raw[2] & 0x8000 != 0 { -w } else { w };
    [x, y, z, w]
}

/// 21 bits per axis and one sign bit for `w`
fn quat64(raw: u64) -> Quat {
    // 1048576.5, not .0: that's what Quaternion64 in compressed_vector.h multiplies by the inverse of, don't ask
    let axis = |shift: u32| (((raw >> shift) & 0x1f_ffff) as i32 - 1_048_576) as f32 * (1.0 / 1_048_576.5);
    let (x, y, z) = (axis(0), axis(21), axis(42));
    let w = (1.0 - x * x - y * y - z * z).max(0.0).sqrt();
    let w = if raw >> 63 != 0 { -w } else { w };
    [x, y, z, w]
}

/// Source's `float16`, used for `Vector48`: IEEE half, except infinity reads as ±65504 and NaN as 0
fn f16(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let mantissa = f32::from(bits & 0x03ff);
    match exponent {
        0 => sign * mantissa * 2.0f32.powi(-24),
        // Convert16bitFloatTo32bits clamps instead of keeping inf/NaN, so a broken position stays finite
        31 => {
            if mantissa == 0.0 {
                sign * 65504.0
            } else {
                0.0
            }
        }
        _ => sign * (1.0 + mantissa / 1024.0) * 2.0f32.powi(exponent - 15),
    }
}
