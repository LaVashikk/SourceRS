//! Placement of a `func_instance`: rotation by its `angles`, then translation by its `origin`

use std::ops::Mul;

use crate::vmf::entities::Entity;
use crate::vmf::world::{Side, Solid};

/// Rotation followed by translation, in Source's axis conventions
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub origin: [f64; 3],
    /// Columns are forward, left and up, like Source's `matrix3x4_t`
    pub rot: [[f64; 3]; 3],
}

impl Transform {
    pub const IDENTITY: Self = Self {
        origin: [0.0; 3],
        rot: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };

    /// Reads the entity's `origin` and `angles` keys, missing ones count as zero
    pub fn from_entity(ent: &Entity) -> Self {
        Self {
            origin: vec3(ent, "origin"),
            rot: angle_matrix(vec3(ent, "angles")),
        }
    }

    /// Maps a point from local space to the parent space
    pub fn point(&self, p: [f64; 3]) -> [f64; 3] {
        let r = self.rotate(p);
        std::array::from_fn(|i| round(r[i] + self.origin[i]))
    }

    /// Rotates a direction, translation doesn't apply
    pub fn vector(&self, v: [f64; 3]) -> [f64; 3] {
        self.rotate(v).map(round)
    }

    /// Maps entity `angles` (pitch yaw roll, in degrees) from local space to the parent space
    pub fn angles(&self, local: [f64; 3]) -> [f64; 3] {
        matrix_angles(mat_mul(&self.rot, &angle_matrix(local))).map(round)
    }

    /// Maps the parent space back to local space
    pub fn inverse(&self) -> Self {
        let rot = std::array::from_fn(|i| std::array::from_fn(|j| self.rot[j][i]));
        let back = Self { origin: [0.0; 3], rot };
        Self {
            origin: back.vector(self.origin.map(|v| -v)),
            rot,
        }
    }

    /// Moves a brush: plane points, texture axes and displacement data
    pub fn apply_solid(&self, solid: &mut Solid) {
        // didn't move? then not a single byte gets reformatted either
        if *self == Self::IDENTITY {
            return;
        }
        for side in &mut solid.sides {
            self.apply_side(side);
        }
    }

    /// Moves an entity's `origin`, turns its `angles` and moves its brushes.
    /// Other keys that only the FGD knows to be positions or angles (`movedir`, the `pitch` of lights) stay as they are
    pub fn apply_entity(&self, ent: &mut Entity) {
        if *self == Self::IDENTITY {
            return;
        }

        if ent.get("origin").is_some() {
            let origin = self.point(vec3(ent, "origin"));
            ent.set("origin".to_string(), fmt_vec3(origin));
        }

        // VBSP doesn't touch angles of an unrotated instance either, no "0 270 0" -> "0 -90 0" churn
        if ent.get("angles").is_some() && self.rot != Self::IDENTITY.rot {
            let angles = self.angles(vec3(ent, "angles"));
            ent.set("angles".to_string(), fmt_vec3(angles));
        }

        for solid in ent.solids.iter_mut().flatten() {
            self.apply_solid(solid);
        }
    }

    fn apply_side(&self, side: &mut Side) {
        if let Some(p) = parse::<9>(&side.plane) {
            let [a, b, c] = [0, 3, 6].map(|i| fmt_vec3(self.point([p[i], p[i + 1], p[i + 2]])));
            side.plane = format!("({a}) ({b}) ({c})");
        }
        self.apply_axis(&mut side.u_axis);
        self.apply_axis(&mut side.v_axis);

        let Some(disp) = &mut side.dispinfo else {
            return;
        };

        if let Some(start) = parse::<3>(&disp.start_position) {
            disp.start_position = format!("[{}]", fmt_vec3(self.point(start)));
        }

        // VBSP moves startposition and NEVER turns these (MergeBrushSides), so disps in rotated instances should come out bent in game prob?
        // Hammer draws them turned, we go with Hammer
        for rows in [&mut disp.normals, &mut disp.offsets, &mut disp.offset_normals] {
            for row in &mut rows.rows {
                self.apply_row(row);
            }
        }
    }

    /// `[x y z shift] scale`, a point lands on texel `dot(p, xyz) / scale + shift`
    fn apply_axis(&self, axis: &mut String) {
        let Some([x, y, z, shift, scale]) = parse::<5>(axis).filter(|&[.., scale]| scale != 0.0) else {
            return;
        };
        let dir = self.vector([x, y, z]);
        
        // shift is in TEXELS, so the origin has to go through `/ scale` too, same as VBSP MergeBrushSides
        let shift = round(shift - dot(self.origin, dir) / scale);
        *axis = format!("[{} {shift}] {scale}", fmt_vec3(dir));
    }

    fn apply_row(&self, row: &mut String) {
        let values: Option<Vec<f64>> = row.split_whitespace().map(|v| v.parse().ok()).collect();
        let Some(values) = values.filter(|v| v.len() % 3 == 0) else {
            return;
        };

        *row = values
            .chunks_exact(3)
            .map(|v| fmt_vec3(self.vector([v[0], v[1], v[2]])))
            .collect::<Vec<_>>()
            .join(" ");
    }

    fn rotate(&self, v: [f64; 3]) -> [f64; 3] {
        self.rot.map(|row| dot(row, v))
    }
}

/// `parent * child` maps points from the child's local space straight to the parent's parent space
impl Mul for Transform {
    type Output = Self;

    fn mul(self, child: Self) -> Self {
        Self {
            origin: self.point(child.origin),
            rot: mat_mul(&self.rot, &child.rot),
        }
    }
}

fn round(v: f64) -> f64 {
    (v * 1e6).round() / 1e6 + 0.0
}

fn vec3(ent: &Entity, key: &str) -> [f64; 3] {
    let mut out = [0.0; 3];
    if let Some(value) = ent.get(key) {
        for (slot, part) in out.iter_mut().zip(value.split_whitespace()) {
            *slot = part.parse().unwrap_or(0.0);
        }
    }

    out
}

/// Exactly `N` numbers of a VMF value, brackets ignored: `(0 64 64) (64 64 64) (64 0 64)`, `[1 0 0 0] 0.25`
fn parse<const N: usize>(value: &str) -> Option<[f64; N]> {
    let mut parts = value
        .split(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | '[' | ']'))
        .filter(|part| !part.is_empty());
    let mut out = [0.0; N];
    
    for slot in &mut out {
        *slot = parts.next()?.parse().ok()?;
    }

    parts.next().is_none().then_some(out)
}

fn fmt_vec3(v: [f64; 3]) -> String {
    format!("{} {} {}", v[0], v[1], v[2])
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn mat_mul(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}

/// Port of mathlib `AngleMatrix`
fn angle_matrix([pitch, yaw, roll]: [f64; 3]) -> [[f64; 3]; 3] {
    let (sp, cp) = pitch.to_radians().sin_cos();
    let (sy, cy) = yaw.to_radians().sin_cos();
    let (sr, cr) = roll.to_radians().sin_cos();

    [
        [cp * cy, sp * sr * cy - cr * sy, sp * cr * cy + sr * sy],
        [cp * sy, sp * sr * sy + cr * cy, sp * cr * sy - sr * cy],
        [-sp, sr * cp, cr * cp],
    ]
}

/// Port of mathlib `MatrixAngles`
fn matrix_angles(m: [[f64; 3]; 3]) -> [f64; 3] {
    let xy = m[0][0].hypot(m[1][0]);
    let pitch = (-m[2][0]).atan2(xy);

    let (yaw, roll) = if xy > 0.001 {
        (m[1][0].atan2(m[0][0]), m[2][1].atan2(m[2][2]))
    } else {
        ((-m[0][1]).atan2(m[1][1]), 0.0)
    };
    
    [pitch, yaw, roll].map(f64::to_degrees)
}
