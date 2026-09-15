//! This module contains the core data structures for representing VMF files,
//! including the `World`, `Entity`, `Solid`, and other related types.
//! It also re-exports the submodules `common`, `entities`, `metadata`, `regions`, and `world`.

pub mod common;
pub mod connection;
pub mod entities;
pub mod file;
pub mod index;
pub mod metadata;
pub mod regions;
pub mod world;

pub use file::VmfFile;
pub use index::{EntId, EntityIndex};
