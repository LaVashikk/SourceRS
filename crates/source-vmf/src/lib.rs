//! A library for parsing and manipulating Valve Map Format (VMF) files.
//!
//! This library provides functionality to parse VMF files used in Source Engine games
//! into Rust data structures, modify the data, and serialize it back into a VMF file.
//!
//! # Example
//!
//! ```
//! use vmf_forge::prelude::*;
//! use std::fs::File;
//!
//! fn main() -> Result<(), VmfError> {
//!     let mut file = File::open("vmf_examples/your_map.vmf")?;
//!     let vmf_file = VmfFile::parse_file(&mut file)?;
//!
//!     println!("Map Version: {}", vmf_file.versioninfo.map_version);
//!
//!     Ok(())
//! }
//! ```

use indexmap::IndexMap;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

pub mod parser;
pub(crate) mod utils;
pub mod vmf;

pub mod errors;
pub mod prelude;

pub use errors::{VmfError, VmfResult};

/// A trait for types that can be serialized into a VMF string representation.
pub trait VmfSerializable {
    /// Serializes the object into a VMF string.
    ///
    /// # Arguments
    ///
    /// * `indent_level` - The indentation level to use for formatting.
    ///
    /// # Returns
    ///
    /// A string representation of the object in VMF format.
    fn to_vmf_string(&self, indent_level: usize) -> String;
}

pub mod vmf_file;
pub use vmf_file::VmfFile;

/// Represents a block in a VMF file, which can contain key-value pairs and other blocks.
#[derive(Debug, Default)]
pub struct VmfBlock {
    /// The name of the block.
    pub name: String,
    /// The key-value pairs in the block.
    pub key_values: IndexMap<String, String>, // what if Cow?!
    /// The child blocks contained within this block.
    pub blocks: Vec<VmfBlock>,
}

impl VmfBlock {
    /// Serializes the `VmfBlock` into a string with the specified indentation level.
    ///
    /// # Arguments
    ///
    /// * `indent_level` - The indentation level to use for formatting.
    ///
    /// # Returns
    ///
    /// A string representation of the `VmfBlock` in VMF format.
    pub fn serialize(&self, indent_level: usize) -> String {
        let indent = "\t".repeat(indent_level);
        let mut output = String::new();

        // Opens the block with its name
        output.push_str(&format!("{}{}\n", indent, self.name));
        output.push_str(&format!("{}{{\n", indent));

        // Adds all key-value pairs with the required indent
        for (key, value) in &self.key_values {
            output.push_str(&format!("{}\t\"{}\" \"{}\"\n", indent, key, value));
        }

        // Adds nested blocks with an increased indentation level
        for block in &self.blocks {
            output.push_str(&block.serialize(indent_level + 1));
        }

        // Closes the block
        output.push_str(&format!("{}}}\n", indent));

        output
    }
}
