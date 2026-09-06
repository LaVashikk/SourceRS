#![forbid(unsafe_code)]

//! Pure Rust access to Valve Texture Format files.
//!
//! The reader supports PC VTF 7.0 through 7.5. It keeps the original bytes in
//! place and returns borrowed views for thumbnails, resources, and individual
//! mip/frame/face/depth subresources.

mod cursor;
mod error;
mod flags;
mod format;
mod header;
mod resource;
mod subresource;
mod vtf;

pub use error::{Error, Result, Section};
pub use flags::TextureFlags;
pub use format::ImageFormat;
pub use header::{Header, Version};
pub use resource::{Resource, ResourceFlags, ResourceLocation, ResourceTag};
pub use subresource::RawImage;
pub use vtf::Vtf;
