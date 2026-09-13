//! This module provides structures for representing the world block in a VMF file, which contains world geometry, hidden entities, and groups.

use indexmap::IndexMap;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

use super::common::Editor;
use crate::utils::{To01String, get_key_ref, take_and_parse_key, take_key_owned};
use crate::{
    VmfBlock, VmfSerializable,
    errors::{VmfError, VmfResult},
};
use std::mem;

/// Represents the world block in a VMF file.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct World {
    /// The key-value pairs associated with the world.
    pub key_values: IndexMap<String, String>,
    /// The list of solids that make up the world geometry.
    pub solids: Vec<Solid>,
    /// The list of hidden solids in the world.
    pub hidden: Vec<Solid>,
    /// The groups present in the world, if any.
    #[cfg_attr(
        feature = "serialization",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub group: Option<Group>,
}

impl TryFrom<VmfBlock> for World {
    type Error = VmfError;

    fn try_from(block: VmfBlock) -> VmfResult<Self> {
        let estimated_solids = block.blocks.len().saturating_sub(1);
        let mut world = World {
            key_values: block.key_values,
            solids: Vec::with_capacity(estimated_solids),
            hidden: Vec::with_capacity(16),
            group: None,
        };

        for mut inner_block in block.blocks {
            match inner_block.name.as_str() {
                "solid" => world.solids.push(Solid::try_from(inner_block)?),
                "group" => world.group = Group::try_from(inner_block).ok(),
                "hidden" => {
                    if !inner_block.blocks.is_empty() {
                        // Take ownership of the first block instead of cloning
                        let hidden_block = mem::take(&mut inner_block.blocks[0]);
                        world.hidden.push(Solid::try_from(hidden_block)?);
                    }
                }
                _ => {
                    // The `world` block does not support other types of blocks (except `hidden`, `group` and `solid`)
                    #[cfg(feature = "debug_assert_info")]
                    debug_assert!(false, "Unexpected block name: {}", inner_block.name);
                }
            };
        }

        Ok(world)
    }
}

impl From<World> for VmfBlock {
    fn from(val: World) -> Self {
        let mut blocks = Vec::new();

        // Add solids
        for solid in val.solids {
            blocks.push(solid.into());
        }

        // Add hidden solids
        for hidden_solid in val.hidden {
            blocks.push(VmfBlock {
                name: "hidden".to_string(),
                key_values: IndexMap::new(),
                blocks: vec![hidden_solid.into()],
            });
        }

        // Add groups
        if let Some(group) = val.group {
            blocks.push(group.into());
        }

        VmfBlock {
            name: "world".to_string(),
            key_values: val.key_values,
            blocks,
        }
    }
}

impl VmfSerializable for World {
    fn to_vmf_string(&self, indent_level: usize) -> String {
        let indent = "\t".repeat(indent_level);
        let mut output = String::with_capacity(2048);

        output.push_str(&format!("{0}world\n{0}{{\n", indent));

        // Adds key_values of the main block
        for (key, value) in &self.key_values {
            output.push_str(&format!("{}\t\"{}\" \"{}\"\n", indent, key, value));
        }

        // Solids Block
        if !self.solids.is_empty() {
            for solid in &self.solids {
                output.push_str(&solid.to_vmf_string(indent_level + 1));
            }
        }

        // Hidden Solids Block
        if !self.hidden.is_empty() {
            output.push_str(&format!("{0}\tHidden\n{0}\t{{\n", indent));
            for solid in &self.hidden {
                output.push_str(&solid.to_vmf_string(indent_level + 2));
            }
            output.push_str(&format!("{}\t}}\n", indent));
        }

        // Group Block
        if let Some(group) = &self.group {
            output.push_str(&group.to_vmf_string(indent_level + 1));
        }

        output.push_str(&format!("{}}}\n", indent));
        output
    }
}

/// Represents a solid object in the VMF world.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Solid {
    /// The unique ID of the solid.
    pub id: u64,
    /// The sides of the solid.
    pub sides: Vec<Side>,
    /// The editor data for the solid.
    pub editor: Editor,
}

impl TryFrom<VmfBlock> for Solid {
    type Error = VmfError;

    fn try_from(mut block: VmfBlock) -> VmfResult<Self> {
        let mut solid = Solid {
            id: take_and_parse_key::<u64>(&mut block.key_values, "id")?,
            sides: Vec::with_capacity(block.blocks.len()),
            ..Default::default()
        };

        for inner_block in block.blocks {
            match inner_block.name.as_str() {
                "side" => solid.sides.push(Side::try_from(inner_block)?),
                "editor" => solid.editor = Editor::try_from(inner_block)?,
                _ => {
                    #[cfg(feature = "debug_assert_info")]
                    debug_assert!(false, "Unexpected block name: {}", inner_block.name);
                }
            }
        }

        Ok(solid)
    }
}

impl From<Solid> for VmfBlock {
    fn from(val: Solid) -> Self {
        let mut blocks = Vec::new();

        // Adds sides
        for side in val.sides {
            blocks.push(side.into());
        }

        // Adds editor
        blocks.push(val.editor.into());

        VmfBlock {
            name: "solid".to_string(),
            key_values: {
                let mut key_values = IndexMap::new();
                key_values.insert("id".to_string(), val.id.to_string());
                key_values
            },
            blocks,
        }
    }
}

impl VmfSerializable for Solid {
    fn to_vmf_string(&self, indent_level: usize) -> String {
        let indent = "\t".repeat(indent_level);
        let mut output = String::with_capacity(256);

        // Start of solid block
        output.push_str(&format!("{0}solid\n{0}{{\n", indent));
        output.push_str(&format!("{}\t\"id\" \"{}\"\n", indent, self.id));

        // Sides
        for side in &self.sides {
            output.push_str(&side.to_vmf_string(indent_level + 1));
        }

        // Editor block
        output.push_str(&self.editor.to_vmf_string(indent_level + 1));

        output.push_str(&format!("{}}}\n", indent));

        output
    }
}

/// Represents a side of a solid object in the VMF world.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Side {
    /// The unique ID of the side.
    pub id: u32,
    /// The plane equation of the side.
    pub plane: String,
    /// The material used on the side.
    pub material: String,
    /// The U axis of the texture coordinates.
    pub u_axis: String,
    /// The V axis of the texture coordinates.
    pub v_axis: String,
    /// The rotation of the texture.
    #[cfg_attr(
        feature = "serialization",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub rotation: Option<f32>,
    /// The scale of the lightmap.
    pub lightmap_scale: u16,
    /// The smoothing groups that this side belongs to.
    pub smoothing_groups: i32,
    /// flags
    #[cfg_attr(
        feature = "serialization",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub flags: Option<u32>,
    /// The displacement info of the side, if any.
    #[cfg_attr(
        feature = "serialization",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub dispinfo: Option<DispInfo>,
}

impl TryFrom<VmfBlock> for Side {
    type Error = VmfError;

    fn try_from(mut block: VmfBlock) -> VmfResult<Self> {
        let kv = &mut block.key_values;
        // Use iter_mut().find() to get a mutable reference without consuming the vector yet
        // We'll use mem::take later if found.
        let dispinfo_block = block.blocks.iter_mut().find(|b| b.name == "dispinfo");

        // Take ownership of required String fields
        let plane = take_key_owned(kv, "plane")?;
        let material = take_key_owned(kv, "material")?;
        let u_axis = take_key_owned(kv, "uaxis")?;
        let v_axis = take_key_owned(kv, "vaxis")?;

        // Parse required numeric fields, taking ownership
        let id = take_and_parse_key::<u32>(kv, "id")?;
        let lightmap_scale = take_and_parse_key::<u16>(kv, "lightmapscale").unwrap_or(16);
        let smoothing_groups = take_and_parse_key::<i32>(kv, "smoothing_groups").unwrap_or(0);

        // Parse optional numeric fields using .ok()
        // This will consume the key if present and parseable, otherwise yield None
        let rotation = take_and_parse_key::<f32>(kv, "rotation").ok();
        let flags = take_and_parse_key::<u32>(kv, "flags").ok();

        let dispinfo = match dispinfo_block {
            Some(block) => Some(DispInfo::try_from(mem::take(block))?),
            None => None,
        };

        Ok(Side {
            id,
            plane,
            material,
            u_axis,
            v_axis,
            rotation,
            lightmap_scale,
            smoothing_groups,
            flags,
            dispinfo,
        })
    }
}

impl From<Side> for VmfBlock {
    fn from(val: Side) -> Self {
        let mut key_values = IndexMap::new();
        key_values.insert("id".to_string(), val.id.to_string());
        key_values.insert("plane".to_string(), val.plane);
        key_values.insert("material".to_string(), val.material);
        key_values.insert("uaxis".to_string(), val.u_axis);
        key_values.insert("vaxis".to_string(), val.v_axis);
        key_values.insert("lightmapscale".to_string(), val.lightmap_scale.to_string());
        key_values.insert(
            "smoothing_groups".to_string(),
            val.smoothing_groups.to_string(),
        );

        if let Some(rotation) = val.rotation {
            key_values.insert("rotation".to_string(), rotation.to_string());
        }
        if let Some(flags) = val.flags {
            key_values.insert("flags".to_string(), flags.to_string());
        }

        let mut blocks = Vec::new();
        if let Some(dispinfo) = val.dispinfo {
            blocks.push(dispinfo.into());
        }

        VmfBlock {
            name: "side".to_string(),
            key_values,
            blocks,
        }
    }
}

impl VmfSerializable for Side {
    fn to_vmf_string(&self, indent_level: usize) -> String {
        let indent = "\t".repeat(indent_level);
        let mut output = String::with_capacity(256);

        // Start of Side block
        output.push_str(&format!("{0}side\n{0}{{\n", indent));

        // Writes all key-value pairs with appropriate indentation
        output.push_str(&format!("{}\t\"id\" \"{}\"\n", indent, self.id));
        output.push_str(&format!("{}\t\"plane\" \"{}\"\n", indent, self.plane));
        output.push_str(&format!("{}\t\"material\" \"{}\"\n", indent, self.material));
        output.push_str(&format!("{}\t\"uaxis\" \"{}\"\n", indent, self.u_axis));
        output.push_str(&format!("{}\t\"vaxis\" \"{}\"\n", indent, self.v_axis));

        if let Some(rotation) = self.rotation {
            output.push_str(&format!("{}\t\"rotation\" \"{}\"\n", indent, rotation));
        }

        output.push_str(&format!(
            "{}\t\"lightmapscale\" \"{}\"\n",
            indent, self.lightmap_scale
        ));
        output.push_str(&format!(
            "{}\t\"smoothing_groups\" \"{}\"\n",
            indent, self.smoothing_groups
        ));

        // Adds the flag if it exists
        if let Some(flags) = self.flags {
            output.push_str(&format!("{}\t\"flags\" \"{}\"\n", indent, flags));
        }

        if let Some(dispinfo) = &self.dispinfo {
            output.push_str(&dispinfo.to_vmf_string(indent_level + 1));
        }

        // End of Side block
        output.push_str(&format!("{0}}}\n", indent));

        output
    }
}

/// Finds a block with the specified name in a vector of `VmfBlock`s,
/// removes it from the vector, and returns ownership.
/// Uses swap_remove for O(1) removal, but changes the order of remaining blocks.
///
/// # Arguments
///
/// * `blocks` - A mutable reference to a vector of `VmfBlock`s to search and modify.
/// * `name` - The name of the block to search for.
///
/// # Returns
///
/// A `Result` containing the owned `VmfBlock` with the specified name,
/// or a `VmfError` if no such block is found.
#[inline(always)]
fn take_block(blocks: &mut Vec<VmfBlock>, name: &str) -> VmfResult<VmfBlock> {
    let index = blocks.iter().position(|b| b.name == name);
    match index {
        // Some(idx) => Ok(mem::take(&mut blocks[idx])),
        Some(idx) => Ok(blocks.swap_remove(idx)),
        None => Err(VmfError::InvalidFormat(format!(
            "Missing {} block in dispinfo",
            name
        ))),
    }
}

/// Represents the displacement information for a side.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct DispInfo {
    /// The power of the displacement map (2, 3, or 4).
    pub power: u8,
    /// The starting position of the displacement.
    pub start_position: String,
    /// Flags for the displacement.
    #[cfg_attr(
        feature = "serialization",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub flags: Option<u32>,
    /// The elevation of the displacement.
    pub elevation: f32,
    /// Whether the displacement is subdivided.
    pub subdiv: bool,
    /// The normals for each vertex in the displacement.
    pub normals: DispRows,
    /// The distances for each vertex in the displacement.
    pub distances: DispRows,
    /// The offsets for each vertex in the displacement.
    pub offsets: DispRows,
    /// The offset normals for each vertex in the displacement.
    pub offset_normals: DispRows,
    /// The alpha values for each vertex in the displacement.
    pub alphas: DispRows,
    /// The triangle tags for the displacement.
    pub triangle_tags: DispRows,
    /// The allowed vertices for the displacement.
    pub allowed_verts: IndexMap<String, Vec<i32>>,
}

impl TryFrom<VmfBlock> for DispInfo {
    type Error = VmfError;

    fn try_from(mut block: VmfBlock) -> VmfResult<Self> {
        // Extract required child blocks first, consuming them from block.blocks
        let normals_block = take_block(&mut block.blocks, "normals")?;
        let distances_block = take_block(&mut block.blocks, "distances")?;
        let alphas_block = take_block(&mut block.blocks, "alphas")?;
        let triangle_tags_block = take_block(&mut block.blocks, "triangle_tags")?;
        let allowed_verts_block = take_block(&mut block.blocks, "allowed_verts")?;

        // These blocks may not be present in the decompiled vmf. Why?
        let offsets = block
            .blocks
            .iter_mut()
            .find(|b| b.name == "offsets")
            .map_or_else(
                || Ok(DispRows::default()),
                |b| DispRows::try_from(mem::take(b)),
            )?;

        let offset_normals = block
            .blocks
            .iter_mut()
            .find(|b| b.name == "offset_normals")
            .map_or_else(
                || Ok(DispRows::default()),
                |b| DispRows::try_from(mem::take(b)),
            )?;

        // Extract key-values from the parent dispinfo block
        let kv = &mut block.key_values;
        let power = take_and_parse_key::<u8>(kv, "power")?;
        let start_position = take_key_owned(kv, "startposition")?;
        let flags = take_and_parse_key::<u32>(kv, "flags").ok();
        let elevation = take_and_parse_key::<f32>(kv, "elevation")?;
        let subdiv = get_key_ref(kv, "subdiv")? == "1";

        // Convert extracted blocks
        let normals = DispRows::try_from(normals_block)?;
        let distances = DispRows::try_from(distances_block)?;
        let alphas = DispRows::try_from(alphas_block)?;
        let triangle_tags = DispRows::try_from(triangle_tags_block)?;
        let allowed_verts = DispInfo::parse_allowed_verts(allowed_verts_block)?;

        Ok(DispInfo {
            power,
            start_position,
            flags,
            elevation,
            subdiv,
            normals,
            distances,
            offsets,
            offset_normals,
            alphas,
            triangle_tags,
            allowed_verts,
        })
    }
}

impl From<DispInfo> for VmfBlock {
    fn from(val: DispInfo) -> Self {
        let blocks = vec![
            val.normals.into_vmf_block("normals"),
            val.distances.into_vmf_block("distances"),
            val.offsets.into_vmf_block("offsets"),
            val.offset_normals.into_vmf_block("offset_normals"),
            val.alphas.into_vmf_block("alphas"),
            val.triangle_tags.into_vmf_block("triangle_tags"),
            DispInfo::allowed_verts_into_vmf_block(val.allowed_verts),
        ];

        let mut key_values = IndexMap::new();
        key_values.insert("power".to_string(), val.power.to_string());
        key_values.insert("startposition".to_string(), val.start_position);
        key_values.insert("elevation".to_string(), val.elevation.to_string());
        key_values.insert("subdiv".to_string(), val.subdiv.to_01_string());

        if let Some(flags) = val.flags {
            key_values.insert("flags".to_string(), flags.to_string());
        }

        VmfBlock {
            name: "dispinfo".to_string(),
            key_values,
            blocks,
        }
    }
}

impl VmfSerializable for DispInfo {
    fn to_vmf_string(&self, indent_level: usize) -> String {
        let indent = "\t".repeat(indent_level);
        let mut output = String::with_capacity(256);

        output.push_str(&format!("{}dispinfo\n", indent));
        output.push_str(&format!("{}{{\n", indent));
        output.push_str(&format!("{}\t\"power\" \"{}\"\n", indent, self.power));
        output.push_str(&format!(
            "{}\t\"startposition\" \"{}\"\n",
            indent, self.start_position
        ));

        // Adds the flag if it exists
        if let Some(flags) = self.flags {
            output.push_str(&format!("{}\t\"flags\" \"{}\"\n", indent, flags));
        }

        output.push_str(&format!(
            "{}\t\"elevation\" \"{}\"\n",
            indent, self.elevation
        ));
        output.push_str(&format!(
            "{}\t\"subdiv\" \"{}\"\n",
            indent,
            self.subdiv.to_01_string()
        ));
        output.push_str(&self.normals.to_vmf_string(indent_level + 1, "normals"));
        output.push_str(&self.distances.to_vmf_string(indent_level + 1, "distances"));
        output.push_str(&self.offsets.to_vmf_string(indent_level + 1, "offsets"));
        output.push_str(
            &self
                .offset_normals
                .to_vmf_string(indent_level + 1, "offset_normals"),
        );
        output.push_str(&self.alphas.to_vmf_string(indent_level + 1, "alphas"));
        output.push_str(
            &self
                .triangle_tags
                .to_vmf_string(indent_level + 1, "triangle_tags"),
        );
        output.push_str(&Self::allowed_verts_to_vmf_string(
            &self.allowed_verts,
            indent_level + 1,
        ));
        output.push_str(&format!("{}}}\n", indent));

        output
    }
}

impl DispInfo {
    /// Parses the allowed vertices from a `VmfBlock`.
    ///
    /// # Arguments
    ///
    /// * `block` - The `VmfBlock` containing the allowed vertices data.
    ///
    /// # Returns
    ///
    /// A `Result` containing an `IndexMap` of allowed vertices, or a `VmfError` if parsing fails.
    fn parse_allowed_verts(block: VmfBlock) -> VmfResult<IndexMap<String, Vec<i32>>> {
        let mut allowed_verts = IndexMap::new();
        for (key, value) in block.key_values.into_iter() {
            let verts: VmfResult<Vec<i32>> = value
                .split_whitespace()
                .map(|s| {
                    s.parse::<i32>().map_err(|e| VmfError::ParseInt {
                        source: e,
                        key: s.to_string(),
                    })
                })
                .collect();
            allowed_verts.insert(key, verts?);
        }
        Ok(allowed_verts)
    }

    /// Converts the allowed vertices data into a `VmfBlock`.
    ///
    /// # Arguments
    ///
    /// * `allowed_verts` - The `IndexMap` containing the allowed vertices data.
    ///
    /// # Returns
    ///
    /// A `VmfBlock` representing the allowed vertices data.
    fn allowed_verts_into_vmf_block(allowed_verts: IndexMap<String, Vec<i32>>) -> VmfBlock {
        let mut key_values = IndexMap::new();
        for (key, values) in allowed_verts {
            key_values.insert(
                key,
                values
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join(" "),
            );
        }

        VmfBlock {
            name: "allowed_verts".to_string(),
            key_values,
            blocks: Vec::new(),
        }
    }

    /// Converts the allowed vertices data into a string representation.
    ///
    /// # Arguments
    ///
    /// * `allowed_verts` - A reference to an `IndexMap` containing the allowed vertices data.
    /// * `indent_level` - The indentation level for formatting.
    ///
    /// # Returns
    ///
    /// A string representation of the allowed vertices data.
    fn allowed_verts_to_vmf_string(
        allowed_verts: &IndexMap<String, Vec<i32>>,
        indent_level: usize,
    ) -> String {
        let indent = "\t".repeat(indent_level);
        let mut output = String::new();

        output.push_str(&format!("{}allowed_verts\n", indent));
        output.push_str(&format!("{}{{\n", indent));
        for (key, values) in allowed_verts {
            output.push_str(&format!(
                "{}\t\"{}\" \"{}\"\n",
                indent,
                key,
                values
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join(" ")
            ));
        }
        output.push_str(&format!("{}}}\n", indent));

        output
    }
}

/// Represents rows of data for displacement information, such as normals, distances, offsets, etc.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct DispRows {
    /// The rows of data, each represented as a string.
    pub rows: Vec<String>,
}

impl TryFrom<VmfBlock> for DispRows {
    type Error = VmfError;

    fn try_from(block: VmfBlock) -> VmfResult<Self> {
        let mut rows = Vec::with_capacity(block.key_values.len());
        for (key, value) in block.key_values {
            if let Some(stripped_idx) = key.strip_prefix("row") {
                let index = stripped_idx
                    .parse::<usize>()
                    .map_err(|e| VmfError::ParseInt {
                        source: e,
                        key: key.to_string(),
                    })?;
                if index >= rows.len() {
                    rows.resize(index + 1, String::new());
                }
                rows[index] = value;
            }
        }
        Ok(DispRows { rows })
    }
}

impl DispRows {
    /// Converts the `DispRows` data into a `VmfBlock` with the specified name.
    ///
    /// # Arguments
    ///
    /// * `self` - The `DispRows` instance to convert.
    /// * `name` - The name of the block.
    ///
    /// # Returns
    ///
    /// A `VmfBlock` representing the `DispRows` data.
    fn into_vmf_block(self, name: &str) -> VmfBlock {
        let mut key_values = IndexMap::new();
        for (i, row) in self.rows.into_iter().enumerate() {
            key_values.insert(format!("row{}", i), row);
        }

        VmfBlock {
            name: name.to_string(),
            key_values,
            blocks: Vec::new(),
        }
    }

    /// Converts the `DispRows` data into a string representation with the specified name and indentation level.
    ///
    /// # Arguments
    ///
    /// * `self` - A reference to the `DispRows` instance.
    /// * `indent_level` - The indentation level for formatting.
    /// * `name` - The name of the block.
    ///
    /// # Returns
    ///
    /// A string representation of the `DispRows` data.
    fn to_vmf_string(&self, indent_level: usize, name: &str) -> String {
        let indent = "\t".repeat(indent_level);
        let mut output = String::with_capacity(32);

        output.push_str(&format!("{}{}\n", indent, name));
        output.push_str(&format!("{}{{\n", indent));
        for (i, row) in self.rows.iter().enumerate() {
            output.push_str(&format!("{}\t\"row{}\" \"{}\"\n", indent, i, row));
        }
        output.push_str(&format!("{}}}\n", indent));

        output
    }
}

/// Represents a group in the VMF world.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Group {
    /// The unique ID of the group.
    pub id: u32,
    /// The editor data for the group.
    pub editor: Editor,
}

impl TryFrom<VmfBlock> for Group {
    type Error = VmfError;

    fn try_from(mut block: VmfBlock) -> VmfResult<Self> {
        let mut editor = None;
        for inner_block in block.blocks {
            if inner_block.name.eq_ignore_ascii_case("editor") {
                editor = Some(Editor::try_from(inner_block)?);
            }
        }

        Ok(Self {
            id: take_and_parse_key::<u32>(&mut block.key_values, "id")?,
            editor: editor.unwrap_or_default(),
        })
    }
}

impl From<Group> for VmfBlock {
    fn from(val: Group) -> Self {
        let mut blocks = Vec::with_capacity(2);

        // Adds Editor block
        blocks.push(val.editor.into());

        VmfBlock {
            name: "group".to_string(),
            key_values: {
                let mut key_values = IndexMap::new();
                key_values.insert("id".to_string(), val.id.to_string());
                key_values
            },
            blocks,
        }
    }
}

impl VmfSerializable for Group {
    fn to_vmf_string(&self, indent_level: usize) -> String {
        let indent = "\t".repeat(indent_level);
        let mut output = String::with_capacity(64);

        // Writes the main entity block
        output.push_str(&format!("{0}group\n{0}{{\n", indent));
        output.push_str(&format!("{}\t\"id\" \"{}\"\n", indent, self.id));

        // Editor block
        output.push_str(&self.editor.to_vmf_string(indent_level + 1));

        output.push_str(&format!("{}}}\n", indent));

        output
    }
}
