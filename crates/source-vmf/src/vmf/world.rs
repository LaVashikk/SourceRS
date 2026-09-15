//! This module provides structures for representing the world block in a VMF file, which contains world geometry, hidden entities, and groups.

use indexmap::IndexMap;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

use super::common::Editor;
use crate::parse_ctx::{FromBlock, ParseCtx, impl_try_from_block};
use crate::utils::{To01String, take_bool, take_opt_parse, take_parse, take_str};
use crate::{Extras, VmfBlock, VmfSerializable};
use std::mem;

impl_try_from_block!(World, Solid, Side, DispInfo, DispRows, Group);

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
    /// The list of groups present in the world.
    #[cfg_attr(
        feature = "serialization",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub groups: Vec<Group>,
    /// Blocks this struct does not model. Unknown *keys* land in `key_values`.
    #[cfg_attr(feature = "serialization", serde(default, skip_serializing_if = "Extras::is_empty"))]
    pub extra: Extras,
}

impl World {
    /// Iterates over all solids in the world, visible and hidden.
    pub fn iter_solids(&self) -> impl Iterator<Item = &Solid> {
        self.solids.iter().chain(self.hidden.iter())
    }

    /// Mutably iterates over all solids in the world, visible and hidden.
    pub fn iter_solids_mut(&mut self) -> impl Iterator<Item = &mut Solid> {
        self.solids.iter_mut().chain(self.hidden.iter_mut())
    }
}

impl FromBlock for World {
    fn from_block(mut block: VmfBlock, ctx: &mut ParseCtx) -> Self {
        ctx.enter("world");
        let estimated_solids = block.blocks.len().saturating_sub(1);
        let mut world = World {
            key_values: std::mem::take(&mut block.key_values),
            solids: Vec::with_capacity(estimated_solids),
            hidden: Vec::with_capacity(16),
            groups: Vec::new(),
            extra: Extras::default(),
        };

        for mut inner_block in block.blocks.drain(..) {
            let name = inner_block.name.clone();
            if name.eq_ignore_ascii_case("solid") {
                ctx.enter_indexed("solid", world.solids.len());
                world.solids.push(Solid::from_block(inner_block, ctx));
                ctx.leave();
            } else if name.eq_ignore_ascii_case("group") {
                ctx.enter_indexed("group", world.groups.len());
                world.groups.push(Group::from_block(inner_block, ctx));
                ctx.leave();
            } else if name.eq_ignore_ascii_case("hidden") {
                // Read all solids inside the hidden wrapper.
                for hidden_block in mem::take(&mut inner_block.blocks) {
                    ctx.enter_indexed("hidden", world.hidden.len());
                    world.hidden.push(Solid::from_block(hidden_block, ctx));
                    ctx.leave();
                }
            } else {
                ctx.warn(format!("unexpected block '{}', kept as-is", name));
                world.extra.blocks.push(inner_block);
            }
        }

        ctx.leave();
        world
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
        for group in val.groups {
            blocks.push(group.into());
        }
        blocks.extend(val.extra.blocks);

        let mut key_values = val.key_values;
        key_values.extend(val.extra.key_values);

        VmfBlock {
            name: "world".to_string(),
            key_values,
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
            crate::write_key_value(&mut output, &format!("{indent}\t"), key, value);
        }

        // Solids Block
        if !self.solids.is_empty() {
            for solid in &self.solids {
                output.push_str(&solid.to_vmf_string(indent_level + 1));
            }
        }

        // Hidden solids (one wrapper per solid)
        for solid in &self.hidden {
            output.push_str(&format!("{0}\thidden\n{0}\t{{\n", indent));
            output.push_str(&solid.to_vmf_string(indent_level + 2));
            output.push_str(&format!("{}\t}}\n", indent));
        }

        // Group Blocks
        for group in &self.groups {
            output.push_str(&group.to_vmf_string(indent_level + 1));
        }
        self.extra.write_key_values(&mut output, indent_level + 1);
        self.extra.write_blocks(&mut output, indent_level + 1);

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
    /// Keys and blocks this struct does not model.
    #[cfg_attr(feature = "serialization", serde(default, skip_serializing_if = "Extras::is_empty"))]
    pub extra: Extras,
}

impl FromBlock for Solid {
    fn from_block(mut block: VmfBlock, ctx: &mut ParseCtx) -> Self {
        let blocks = std::mem::take(&mut block.blocks);
        let mut solid = Solid {
            id: take_parse(&mut block.key_values, "id", 0, ctx),
            sides: Vec::with_capacity(blocks.len()),
            ..Default::default()
        };

        for inner_block in blocks {
            let name = inner_block.name.clone();
            if name.eq_ignore_ascii_case("side") {
                ctx.enter_indexed("side", solid.sides.len());
                solid.sides.push(Side::from_block(inner_block, ctx));
                ctx.leave();
            } else if name.eq_ignore_ascii_case("editor") {
                ctx.enter("editor");
                solid.editor = Editor::from_block(inner_block, ctx);
                ctx.leave();
            } else {
                ctx.warn(format!("unexpected block '{}', kept as-is", name));
                solid.extra.blocks.push(inner_block);
            }
        }

        solid.extra.key_values = block.key_values;
        solid
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
        blocks.extend(val.extra.blocks);

        VmfBlock {
            name: "solid".to_string(),
            key_values: {
                let mut key_values = IndexMap::new();
                key_values.insert("id".to_string(), val.id.to_string());
                key_values.extend(val.extra.key_values);
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
        self.extra.write_key_values(&mut output, indent_level + 1);

        // Sides
        for side in &self.sides {
            output.push_str(&side.to_vmf_string(indent_level + 1));
        }

        // Editor block
        output.push_str(&self.editor.to_vmf_string(indent_level + 1));
        self.extra.write_blocks(&mut output, indent_level + 1);

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
    /// Keys and blocks this struct does not model.
    #[cfg_attr(feature = "serialization", serde(default, skip_serializing_if = "Extras::is_empty"))]
    pub extra: Extras,
}

impl FromBlock for Side {
    fn from_block(mut block: VmfBlock, ctx: &mut ParseCtx) -> Self {
        let mut dispinfo = None;
        let mut unknown = Vec::new();

        for inner_block in block.blocks.drain(..) {
            if inner_block.name.eq_ignore_ascii_case("dispinfo") {
                ctx.enter("dispinfo");
                dispinfo = Some(DispInfo::from_block(inner_block, ctx));
                ctx.leave();
            } else {
                ctx.warn(format!(
                    "unexpected block '{}', kept as-is",
                    inner_block.name
                ));
                unknown.push(inner_block);
            }
        }

        let kv = &mut block.key_values;

        let side = Side {
            id: take_parse(kv, "id", 0, ctx),
            plane: take_str(kv, "plane", "(0 0 0) (0 0 0) (0 0 0)", ctx),
            material: take_str(kv, "material", "TOOLS/TOOLSNODRAW", ctx),
            u_axis: take_str(kv, "uaxis", "[1 0 0 0] 0.25", ctx),
            v_axis: take_str(kv, "vaxis", "[0 -1 0 0] 0.25", ctx),
            rotation: take_opt_parse::<f32>(kv, "rotation", ctx),
            lightmap_scale: take_opt_parse::<u16>(kv, "lightmapscale", ctx).unwrap_or(16),
            smoothing_groups: take_opt_parse::<i32>(kv, "smoothing_groups", ctx).unwrap_or(0),
            flags: take_opt_parse::<u32>(kv, "flags", ctx),
            dispinfo,
            extra: Extras::default(),
        };

        Side {
            extra: Extras {
                key_values: std::mem::take(kv),
                blocks: unknown,
            },
            ..side
        }
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
        key_values.extend(val.extra.key_values);

        let mut blocks = Vec::new();
        if let Some(dispinfo) = val.dispinfo {
            blocks.push(dispinfo.into());
        }
        blocks.extend(val.extra.blocks);

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
        self.extra.write_key_values(&mut output, indent_level + 1);

        if let Some(dispinfo) = &self.dispinfo {
            output.push_str(&dispinfo.to_vmf_string(indent_level + 1));
        }
        self.extra.write_blocks(&mut output, indent_level + 1);

        // End of Side block
        output.push_str(&format!("{0}}}\n", indent));

        output
    }
}

/// Finds a block by name (case-insensitively) and removes it from the vector.
#[inline]
fn take_block(blocks: &mut Vec<VmfBlock>, name: &str) -> Option<VmfBlock> {
    let index = blocks.iter().position(|b| b.name.eq_ignore_ascii_case(name))?;
    Some(blocks.remove(index))
}

/// Takes a named rows block, or warns and yields an empty one.
fn take_rows(blocks: &mut Vec<VmfBlock>, name: &str, ctx: &mut ParseCtx) -> DispRows {
    match take_block(blocks, name) {
        Some(block) => {
            ctx.enter(name);
            let rows = DispRows::from_block(block, ctx);
            ctx.leave();
            rows
        }
        None => {
            ctx.warn(format!("missing '{}' block, using an empty one", name));
            DispRows::default()
        }
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
    /// Keys and blocks this struct does not model.
    #[cfg_attr(feature = "serialization", serde(default, skip_serializing_if = "Extras::is_empty"))]
    pub extra: Extras,
}

impl FromBlock for DispInfo {
    fn from_block(mut block: VmfBlock, ctx: &mut ParseCtx) -> Self {
        let normals = take_rows(&mut block.blocks, "normals", ctx);
        let distances = take_rows(&mut block.blocks, "distances", ctx);
        let alphas = take_rows(&mut block.blocks, "alphas", ctx);
        let triangle_tags = take_rows(&mut block.blocks, "triangle_tags", ctx);

        let allowed_verts = match take_block(&mut block.blocks, "allowed_verts") {
            Some(b) => DispInfo::parse_allowed_verts(b, ctx),
            None => IndexMap::new(),
        };

        let offsets = take_block(&mut block.blocks, "offsets")
            .map_or_else(DispRows::default, |b| DispRows::from_block(b, ctx));
        let offset_normals = take_block(&mut block.blocks, "offset_normals")
            .map_or_else(DispRows::default, |b| DispRows::from_block(b, ctx));

        let unknown = std::mem::take(&mut block.blocks);
        for leftover in &unknown {
            ctx.warn(format!("unexpected block '{}', kept as-is", leftover.name));
        }

        let kv = &mut block.key_values;
        DispInfo {
            power: take_parse(kv, "power", 3, ctx),
            start_position: take_str(kv, "startposition", "[0 0 0]", ctx),
            flags: take_opt_parse::<u32>(kv, "flags", ctx),
            elevation: take_parse(kv, "elevation", 0.0, ctx),
            subdiv: take_bool(kv, "subdiv", false, ctx),
            normals,
            distances,
            offsets,
            offset_normals,
            alphas,
            triangle_tags,
            allowed_verts,
            extra: Extras {
                key_values: std::mem::take(kv),
                blocks: unknown,
            },
        }
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
        key_values.extend(val.extra.key_values);

        let mut blocks = blocks;
        blocks.extend(val.extra.blocks);

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
        self.extra.write_key_values(&mut output, indent_level + 1);
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
        self.extra.write_blocks(&mut output, indent_level + 1);
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
    fn parse_allowed_verts(block: VmfBlock, ctx: &mut ParseCtx) -> IndexMap<String, Vec<i32>> {
        let mut allowed_verts = IndexMap::new();
        for (key, value) in block.key_values.into_iter() {
            let verts: Vec<i32> = value
                .split_whitespace()
                .filter_map(|s| match s.parse::<i32>() {
                    Ok(v) => Some(v),
                    Err(_) => {
                        ctx.warn(format!("allowed_verts/{}: skipping '{}'", key, s));
                        None
                    }
                })
                .collect();
            allowed_verts.insert(key, verts);
        }
        allowed_verts
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

impl FromBlock for DispRows {
    fn from_block(block: VmfBlock, ctx: &mut ParseCtx) -> Self {
        let mut rows = Vec::with_capacity(block.key_values.len());
        for (key, value) in block.key_values {
            let stripped = key
                .get(..3)
                .filter(|p| p.eq_ignore_ascii_case("row"))
                .map(|_| &key[3..]);

            let Some(stripped_idx) = stripped else {
                ctx.warn(format!("unexpected key '{}', dropped", key));
                continue;
            };

            match stripped_idx.parse::<usize>() {
                Ok(index) => {
                    if index >= rows.len() {
                        rows.resize(index + 1, String::new());
                    }
                    rows[index] = value;
                }
                Err(_) => ctx.warn(format!("key '{}' has no valid row index, dropped", key)),
            }
        }
        DispRows { rows }
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
    /// Keys and blocks this struct does not model.
    #[cfg_attr(feature = "serialization", serde(default, skip_serializing_if = "Extras::is_empty"))]
    pub extra: Extras,
}

impl FromBlock for Group {
    fn from_block(mut block: VmfBlock, ctx: &mut ParseCtx) -> Self {
        let mut editor = None;
        let mut unknown = Vec::new();

        for inner_block in block.blocks.drain(..) {
            if inner_block.name.eq_ignore_ascii_case("editor") {
                ctx.enter("editor");
                editor = Some(Editor::from_block(inner_block, ctx));
                ctx.leave();
            } else {
                ctx.warn(format!(
                    "unexpected block '{}', kept as-is",
                    inner_block.name
                ));
                unknown.push(inner_block);
            }
        }

        let kv = &mut block.key_values;
        Self {
            id: take_parse(kv, "id", 0, ctx),
            editor: editor.unwrap_or_default(),
            extra: Extras {
                key_values: std::mem::take(kv),
                blocks: unknown,
            },
        }
    }
}

impl From<Group> for VmfBlock {
    fn from(val: Group) -> Self {
        let mut blocks = Vec::with_capacity(2);

        // Adds Editor block
        blocks.push(val.editor.into());
        blocks.extend(val.extra.blocks);

        VmfBlock {
            name: "group".to_string(),
            key_values: {
                let mut key_values = IndexMap::new();
                key_values.insert("id".to_string(), val.id.to_string());
                key_values.extend(val.extra.key_values);
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
        self.extra.write_key_values(&mut output, indent_level + 1);

        // Editor block
        output.push_str(&self.editor.to_vmf_string(indent_level + 1));
        self.extra.write_blocks(&mut output, indent_level + 1);

        output.push_str(&format!("{}}}\n", indent));

        output
    }
}
