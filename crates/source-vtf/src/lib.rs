#![forbid(unsafe_code)]

//! Pure Rust access to Valve Texture Format files
//!
//! Reads PC VTF 7.0 through 7.5 without copying: thumbnails, resources and every mip, frame, face and slice are slices of the original bytes

mod cursor;
mod error;
mod header;
mod image;
mod vtf;

pub use error::{Error, Result};
pub use header::{Header, Resource, ResourceLocation, ResourceTag, TextureFlags, Version};
pub use image::{ImageFormat, RawImage};
pub use vtf::Vtf;

/// A [`Vtf`] that owns its underlying byte buffer in heap memory.
///
/// Returned by [`Vtf::open`]. Unlike borrowed [`Vtf`] instances, this owns its file bytes
/// and can be stored in long-lived caches or sent across threads without lifetime parameters.
pub type OwnedVtf = Vtf<'static>;
