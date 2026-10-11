#![forbid(unsafe_code)]

//! Pure Rust access to Source 1 studio models
//!
//! The core parses MDL 44-49, VVD 4 and VTX 7 from bytes or paths, without game search paths or material parsers.
//! The `model_system` feature adds [`ModelSystem`], which loads models and their materials through `source-fs` and `source-vmt`

mod cursor;
mod error;
mod studio;
#[cfg(feature = "model_system")]
mod system;

pub mod animation;
pub mod mdl;
pub mod vtx;
pub mod vvd;

pub use animation::Pose;
pub use error::{Error, FileKind, Result};
pub use mdl::Mdl;
pub use studio::{Geometry, GeometryMesh, GeometryOptions, MaterialReference, StudioModel};
#[cfg(feature = "model_system")]
pub use system::ModelSystem;
pub use vtx::Vtx;
pub use vvd::{Vertex, Vvd};

pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];
pub type Vec4 = [f32; 4];
pub type Quat = [f32; 4];
pub type Mat3x4 = [[f32; 4]; 3];

/// `MAX_NUM_LODS` in the SDK's `studio.h`
pub const MAX_LODS: usize = 8;
