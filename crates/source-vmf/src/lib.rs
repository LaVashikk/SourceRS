//! A library for parsing and manipulating Valve Map Format (VMF) files.
//!
//! This library provides functionality to parse VMF files used in Source Engine games
//! into Rust data structures, modify the data, and serialize it back into a VMF file.
//!
//! # Example
//!
//! ```
//! use source_vmf::prelude::*;
//! use std::fs::File;
//!
//! fn main() -> Result<(), VmfError> {
//!     let vmf_file = VmfFile::open("vmf_examples/your_map.vmf")?;
//!
//!     println!("Map Version: {}", vmf_file.versioninfo.map_version);
//!
//!     Ok(())
//! }
//! ```

use indexmap::IndexMap;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

pub mod parse_ctx;
pub mod parser;
pub(crate) mod utils;
pub mod vmf;

pub mod errors;
pub mod prelude;

pub use errors::{VmfError, VmfResult};
pub use parse_ctx::{FromBlock, ParseCtx};

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

pub use vmf::{EntId, EntityIndex, VmfFile};

/// Preserves unrecognized keys and nested blocks during parsing for lossless round-tripping.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Extras {
    /// Keys that are not mapped to typed struct fields.
    pub key_values: IndexMap<String, String>,
    /// Nested blocks that are not mapped to typed struct fields.
    pub blocks: Vec<VmfBlock>,
}

/// Internal delimiter used to store multiple values of repeated keys in a single map entry.
pub const REPEATED_KEY_SEP: char = '\r';

/// Writes key-value pairs, expanding values joined by [`REPEATED_KEY_SEP`] back into separate lines.
pub fn write_key_value(output: &mut String, indent: &str, key: &str, value: &str) {
    for part in value.split(REPEATED_KEY_SEP) {
        output.push_str(indent);
        output.push('"');
        output.push_str(key);
        output.push_str("\" \"");
        output.push_str(part);
        output.push_str("\"\n");
    }
}

/// Joins multiple values for a repeated key into a single string using [`REPEATED_KEY_SEP`].
pub fn join_repeated_key(values: impl IntoIterator<Item = impl std::fmt::Display>) -> String {
    let mut joined = String::new();
    for value in values {
        if !joined.is_empty() {
            joined.push(REPEATED_KEY_SEP);
        }
        joined.push_str(&value.to_string());
    }
    joined
}

impl Extras {
    pub fn is_empty(&self) -> bool {
        self.key_values.is_empty() && self.blocks.is_empty()
    }

    /// Serializes unrecognized key-value pairs at the specified indentation level.
    pub fn write_key_values(&self, output: &mut String, indent_level: usize) {
        let indent = "\t".repeat(indent_level);
        for (key, value) in &self.key_values {
            write_key_value(output, &indent, key, value);
        }
    }

    /// Serializes unrecognized blocks at the specified indentation level.
    pub fn write_blocks(&self, output: &mut String, indent_level: usize) {
        for block in &self.blocks {
            output.push_str(&block.serialize(indent_level));
        }
    }
}

/// Represents a block in a VMF file, which can contain key-value pairs and other blocks.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
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
            write_key_value(&mut output, &format!("{indent}\t"), key, value);
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
