//! A prelude module that re-exports commonly used items from the `vmf_forge` crate.
//!
//! This module is intended to be used as a convenient way to import the most
//! frequently used items from the crate, without having to specify the full path
//! to each item.
//!
//! # Example
//!
//! ```
//! use vmf_forge::prelude::*;
//!
//! // Now you can use VmfFile, VmfError, etc. without the need for crate::
//! let vmf_file = VmfFile::open("your_map.vmf");
//! ```

pub use crate::VmfFile;

pub use crate::errors::{VmfError, VmfResult};

pub use crate::vmf::{
    common::Editor,
    entities::{Entities, Entity},
    metadata::{VersionInfo, ViewSettings, VisGroup, VisGroups},
    regions::{Camera, Cameras, Cordon, Cordons},
    world::{Side, Solid, World},
};
