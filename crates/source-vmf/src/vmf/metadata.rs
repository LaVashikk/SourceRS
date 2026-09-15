//! This module provides structures for representing metadata blocks in a VMF file, such as version info, visgroups, and view settings.

use std::collections::HashSet;

use derive_more::{Deref, DerefMut, IntoIterator};

use indexmap::IndexMap;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

use crate::parse_ctx::{FromBlock, ParseCtx, impl_try_from_block};
use crate::utils::{To01String, take_bool, take_parse, take_str};
use crate::{Extras, VmfBlock, VmfSerializable};

impl_try_from_block!(VersionInfo, VisGroups, VisGroup, ViewSettings);

/// Represents the version info of a VMF file.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct VersionInfo {
    /// The editor version.
    pub editor_version: i32,
    /// The editor build number.
    pub editor_build: i32,
    /// The map version.
    pub map_version: i32,
    /// The format version.
    pub format_version: i32,
    /// Whether the VMF is a prefab.
    pub prefab: bool,
    /// Keys and blocks this struct does not model.
    #[cfg_attr(feature = "serialization", serde(default, skip_serializing_if = "Extras::is_empty"))]
    pub extra: Extras,
}

impl FromBlock for VersionInfo {
    fn from_block(mut block: VmfBlock, ctx: &mut ParseCtx) -> Self {
        ctx.enter("versioninfo");
        let blocks = std::mem::take(&mut block.blocks);
        let kv = &mut block.key_values;
        let d = Self::default();
        let info = Self {
            editor_version: take_parse(kv, "editorversion", d.editor_version, ctx),
            editor_build: take_parse(kv, "editorbuild", d.editor_build, ctx),
            map_version: take_parse(kv, "mapversion", d.map_version, ctx),
            format_version: take_parse(kv, "formatversion", d.format_version, ctx),
            prefab: take_bool(kv, "prefab", d.prefab, ctx),
            extra: Extras {
                key_values: std::mem::take(kv),
                blocks,
            },
        };
        ctx.leave();
        info
    }
}

impl From<VersionInfo> for VmfBlock {
    fn from(val: VersionInfo) -> Self {
        let mut key_values = IndexMap::new();
        key_values.insert("editorversion".to_string(), val.editor_version.to_string());
        key_values.insert("editorbuild".to_string(), val.editor_build.to_string());
        key_values.insert("mapversion".to_string(), val.map_version.to_string());
        key_values.insert("formatversion".to_string(), val.format_version.to_string());
        key_values.insert("prefab".to_string(), val.prefab.to_01_string());
        key_values.extend(val.extra.key_values);

        VmfBlock {
            name: "versioninfo".to_string(),
            key_values,
            blocks: val.extra.blocks,
        }
    }
}

impl VmfSerializable for VersionInfo {
    fn to_vmf_string(&self, indent_level: usize) -> String {
        let indent = "\t".repeat(indent_level);
        let mut output = String::with_capacity(256);

        output.push_str(&format!("{0}versioninfo\n{0}{{\n", indent));
        output.push_str(&format!(
            "{}\t\"editorversion\" \"{}\"\n",
            indent, self.editor_version
        ));
        output.push_str(&format!(
            "{}\t\"editorbuild\" \"{}\"\n",
            indent, self.editor_build
        ));
        output.push_str(&format!(
            "{}\t\"mapversion\" \"{}\"\n",
            indent, self.map_version
        ));
        output.push_str(&format!(
            "{}\t\"formatversion\" \"{}\"\n",
            indent, self.format_version
        ));
        output.push_str(&format!(
            "{}\t\"prefab\" \"{}\"\n",
            indent,
            self.prefab.to_01_string()
        ));
        self.extra.write_key_values(&mut output, indent_level + 1);
        self.extra.write_blocks(&mut output, indent_level + 1);

        output.push_str(&format!("{}}}\n", indent));
        output
    }
}

/// Represents a collection of VisGroups in a VMF file.
#[derive(Debug, Default, Clone, PartialEq, Deref, DerefMut, IntoIterator)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct VisGroups {
    /// The list of VisGroups.
    #[deref]
    #[deref_mut]
    #[into_iterator]
    pub groups: Vec<VisGroup>,
    /// Keys and blocks this struct does not model.
    #[cfg_attr(feature = "serialization", serde(default, skip_serializing_if = "Extras::is_empty"))]
    pub extra: Extras,
}

/// Recursively finds a VisGroup by its ID within a slice of VisGroups.
/// Returns None if not found.
fn find_visgroup_by_id(groups: &[VisGroup], id_to_find: i32) -> Option<&VisGroup> {
    for group in groups {
        if group.id == id_to_find {
            return Some(group);
        }
        if let Some(ref children) = group.children
            && let Some(found) = find_visgroup_by_id(children, id_to_find) {
                return Some(found);
            }
    }
    None
}

/// Recursively finds a mutable reference to a VisGroup by its ID within a slice of VisGroups.
/// Returns None if not found.
fn find_visgroup_by_id_mut(
    groups: &mut [VisGroup],
    id_to_find: i32,
) -> Option<&mut VisGroup> {
    for group in groups {
        if group.id == id_to_find {
            return Some(group);
        }
        if let Some(ref mut children) = group.children
            && let Some(found) = find_visgroup_by_id_mut(children, id_to_find) {
                return Some(found);
            }
    }
    None
}

/// Recursively finds a VisGroup by its name within a slice of VisGroups.
/// Returns None if not found.
fn find_visgroup_by_name<'a>(groups: &'a [VisGroup], name_to_find: &str) -> Option<&'a VisGroup> {
    for group in groups {
        if group.name == name_to_find {
            return Some(group);
        }
        if let Some(ref children) = group.children
            && let Some(found) = find_visgroup_by_name(children, name_to_find) {
                return Some(found);
            }
    }
    None
}

/// Recursively finds a mutable reference to a VisGroup by its name within a slice of VisGroups.
/// Returns None if not found.
fn find_visgroup_by_name_mut<'a>(
    groups: &'a mut [VisGroup],
    name_to_find: &str,
) -> Option<&'a mut VisGroup> {
    for group in groups {
        if group.name == name_to_find {
            return Some(group);
        }
        if let Some(ref mut children) = group.children
            && let Some(found) = find_visgroup_by_name_mut(children, name_to_find) {
                return Some(found);
            }
    }
    None
}

impl VisGroups {
    /// Finds a VisGroup by its name recursively within this collection.
    ///
    /// # Arguments
    ///
    /// * `name` - The exact name of the VisGroup to find.
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to the found `VisGroup`, or `None`.
    pub fn find_by_name(&self, name: &str) -> Option<&VisGroup> {
        find_visgroup_by_name(&self.groups, name)
    }

    /// Finds a mutable reference to a VisGroup by its name recursively within this collection.
    ///
    /// # Arguments
    ///
    /// * `name` - The exact name of the VisGroup to find.
    ///
    /// # Returns
    ///
    /// An `Option` containing a mutable reference to the found `VisGroup`, or `None`.
    pub fn find_by_name_mut(&mut self, name: &str) -> Option<&mut VisGroup> {
        find_visgroup_by_name_mut(&mut self.groups, name)
    }

    /// Finds a VisGroup by its ID recursively within this collection.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the VisGroup to find.
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to the found `VisGroup`, or `None`.
    pub fn find_by_id(&self, id: i32) -> Option<&VisGroup> {
        find_visgroup_by_id(&self.groups, id)
    }

    /// Finds a mutable reference to a VisGroup by its ID recursively within this collection.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the VisGroup to find.
    ///
    /// # Returns
    ///
    /// An `Option` containing a mutable reference to the found `VisGroup`, or `None`.
    pub fn find_by_id_mut(&mut self, id: i32) -> Option<&mut VisGroup> {
        find_visgroup_by_id_mut(&mut self.groups, id)
    }

    /// Returns an unused VisGroup ID (one higher than the current maximum).
    pub fn next_id(&self) -> i32 {
        fn highest(groups: &[VisGroup]) -> i32 {
            groups.iter().fold(0, |max, group| {
                max.max(group.id)
                    .max(group.children.as_deref().map_or(0, highest))
            })
        }
        highest(&self.groups) + 1
    }

    /// Adds a top-level visgroup and returns its ID.
    pub fn create(&mut self, name: impl Into<String>) -> i32 {
        let id = self.next_id();
        self.groups.push(VisGroup {
            name: name.into(),
            id,
            color: "255 255 255".to_string(),
            children: None,
            extra: Extras::default(),
        });
        id
    }

    /// Removes a visgroup and all its children by ID.
    ///
    /// Does not update entity or solid memberships; see [`Editor::leave_visgroup`].
    ///
    /// [`Editor::leave_visgroup`]: crate::vmf::common::Editor::leave_visgroup
    pub fn remove_by_id(&mut self, id: i32) -> Option<VisGroup> {
        fn remove(groups: &mut Vec<VisGroup>, id: i32) -> Option<VisGroup> {
            if let Some(at) = groups.iter().position(|group| group.id == id) {
                return Some(groups.remove(at));
            }
            for group in groups {
                let Some(children) = group.children.as_mut() else {
                    continue;
                };
                if let Some(found) = remove(children, id) {
                    if children.is_empty() {
                        group.children = None;
                    }
                    return Some(found);
                }
            }
            None
        }
        remove(&mut self.groups, id)
    }

    /// Resolves all IDs for a given VisGroup ID, optionally including all descendant IDs.
    pub fn resolve_ids(&self, group_id: i32, include_children: bool) -> Option<HashSet<i32>> {
        let start_group = self.find_by_id(group_id)?;
        let mut ids = HashSet::new();
        if include_children {
            start_group.collect_ids(&mut ids);
        } else {
            ids.insert(start_group.id);
        }
        Some(ids)
    }
}

impl FromBlock for VisGroups {
    fn from_block(mut block: VmfBlock, ctx: &mut ParseCtx) -> Self {
        ctx.enter("visgroups");
        let mut groups = Vec::with_capacity(block.blocks.len());
        let mut unknown = Vec::new();

        for (index, group) in block.blocks.drain(..).enumerate() {
            if group.name.eq_ignore_ascii_case("visgroup") {
                ctx.enter_indexed("visgroup", index);
                groups.push(VisGroup::from_block(group, ctx));
                ctx.leave();
            } else {
                ctx.warn(format!("unexpected block '{}', kept as-is", group.name));
                unknown.push(group);
            }
        }

        ctx.leave();
        Self {
            groups,
            extra: Extras {
                key_values: block.key_values,
                blocks: unknown,
            },
        }
    }
}

impl From<VisGroups> for VmfBlock {
    fn from(val: VisGroups) -> Self {
        let mut visgroups_block = VmfBlock {
            name: "visgroups".to_string(),
            key_values: val.extra.key_values,
            blocks: Vec::with_capacity(val.groups.len() + val.extra.blocks.len()),
        };

        for group in val.groups {
            visgroups_block.blocks.push(group.into())
        }
        visgroups_block.blocks.extend(val.extra.blocks);

        visgroups_block
    }
}

impl VmfSerializable for VisGroups {
    fn to_vmf_string(&self, indent_level: usize) -> String {
        let indent = "\t".repeat(indent_level);
        let mut output = String::with_capacity(128);

        output.push_str(&format!("{0}visgroups\n{0}{{\n", indent));
        self.extra.write_key_values(&mut output, indent_level + 1);

        for group in &self.groups {
            output.push_str(&group.to_vmf_string(indent_level));
        }
        self.extra.write_blocks(&mut output, indent_level + 1);

        output.push_str(&format!("{}}}\n", indent));
        output
    }
}

/// Represents a VisGroup in a VMF file.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct VisGroup {
    /// The name of the VisGroup.
    pub name: String,
    /// The ID of the VisGroup.
    pub id: i32,
    /// The color of the VisGroup in the editor.
    pub color: String,
    /// The child VisGroups of this VisGroup, if any.
    #[cfg_attr(
        feature = "serialization",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub children: Option<Vec<VisGroup>>,
    /// Keys and blocks this struct does not model.
    #[cfg_attr(feature = "serialization", serde(default, skip_serializing_if = "Extras::is_empty"))]
    pub extra: Extras,
}

impl VisGroup {
    /// Recursively collects this group's ID and all its descendants' IDs into a set.
    pub fn collect_ids(&self, ids: &mut HashSet<i32>) {
        if !ids.insert(self.id) {
            return;
        }
        if let Some(ref children) = self.children {
            for child in children {
                child.collect_ids(ids);
            }
        }
    }
}

impl FromBlock for VisGroup {
    fn from_block(mut block: VmfBlock, ctx: &mut ParseCtx) -> Self {
        let mut children_vec = Vec::with_capacity(block.blocks.len());
        let mut unknown = Vec::new();

        for (index, child_block) in block.blocks.drain(..).enumerate() {
            if child_block.name.eq_ignore_ascii_case("visgroup") {
                ctx.enter_indexed("visgroup", index);
                children_vec.push(VisGroup::from_block(child_block, ctx));
                ctx.leave();
            } else {
                ctx.warn(format!("unexpected block '{}', kept as-is", child_block.name));
                unknown.push(child_block);
            }
        }

        let kv = &mut block.key_values;
        Self {
            name: take_str(kv, "name", "", ctx),
            id: take_parse(kv, "visgroupid", 0, ctx),
            color: take_str(kv, "color", "255 255 255", ctx),
            children: (!children_vec.is_empty()).then_some(children_vec),
            extra: Extras {
                key_values: std::mem::take(kv),
                blocks: unknown,
            },
        }
    }
}

impl From<VisGroup> for VmfBlock {
    fn from(val: VisGroup) -> Self {
        // Create a block for VisGroup
        let mut visgroup_block = VmfBlock {
            name: "visgroup".to_string(),
            key_values: IndexMap::new(),
            blocks: Vec::new(),
        };

        // Adds key-value pairs for VisGroup
        visgroup_block
            .key_values
            .insert("name".to_string(), val.name);
        visgroup_block
            .key_values
            .insert("visgroupid".to_string(), val.id.to_string());
        visgroup_block
            .key_values
            .insert("color".to_string(), val.color);
        visgroup_block.key_values.extend(val.extra.key_values);

        // If the `VisGroup` has a child element, adds it as nested block
        if let Some(children) = val.children {
            for child in children {
                visgroup_block.blocks.push(child.into());
            }
        }
        visgroup_block.blocks.extend(val.extra.blocks);

        visgroup_block
    }
}

impl VmfSerializable for VisGroup {
    fn to_vmf_string(&self, indent_level: usize) -> String {
        let indent = "\t".repeat(indent_level);
        let mut output = String::with_capacity(64);

        output.push_str(&format!("{0}\tvisgroup\n\t{0}{{\n", indent));
        output.push_str(&format!("{}\t\t\"name\" \"{}\"\n", indent, self.name));
        output.push_str(&format!("{}\t\t\"visgroupid\" \"{}\"\n", indent, self.id));
        output.push_str(&format!("{}\t\t\"color\" \"{}\"\n", indent, self.color));
        self.extra.write_key_values(&mut output, indent_level + 2);

        // If there are child elements, adds them
        if let Some(ref children) = self.children {
            for child in children {
                output.push_str(&child.to_vmf_string(indent_level + 1));
            }
        }
        self.extra.write_blocks(&mut output, indent_level + 2);

        output.push_str(&format!("{}\t}}\n", indent));
        output
    }
}

/// Represents the view settings of a VMF file.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct ViewSettings {
    /// Whether snapping to the grid is enabled.
    pub snap_to_grid: bool,
    /// Whether the grid is shown in the editor.
    pub show_grid: bool,
    /// Whether the logical grid is shown in the editor.
    pub show_logical_grid: bool,
    /// The grid spacing.
    pub grid_spacing: u16,
    /// Whether the 3D grid is shown in the editor.
    pub show_3d_grid: bool,
    /// Keys and blocks this struct does not model.
    #[cfg_attr(feature = "serialization", serde(default, skip_serializing_if = "Extras::is_empty"))]
    pub extra: Extras,
}

impl Default for ViewSettings {
    fn default() -> Self {
        Self {
            snap_to_grid: true,
            show_grid: true,
            show_logical_grid: false,
            grid_spacing: 8,
            show_3d_grid: false,
            extra: Extras::default(),
        }
    }
}

impl FromBlock for ViewSettings {
    fn from_block(mut block: VmfBlock, ctx: &mut ParseCtx) -> Self {
        ctx.enter("viewsettings");
        let blocks = std::mem::take(&mut block.blocks);
        let kv = &mut block.key_values;
        let d = Self::default();

        let settings = Self {
            snap_to_grid: take_bool(kv, "bSnapToGrid", d.snap_to_grid, ctx),
            show_grid: take_bool(kv, "bShowGrid", d.show_grid, ctx),
            show_logical_grid: take_bool(kv, "bShowLogicalGrid", d.show_logical_grid, ctx),
            grid_spacing: take_parse(kv, "nGridSpacing", d.grid_spacing, ctx),
            show_3d_grid: take_bool(kv, "bShow3DGrid", d.show_3d_grid, ctx),
            extra: Extras {
                key_values: std::mem::take(kv),
                blocks,
            },
        };
        ctx.leave();
        settings
    }
}

impl From<ViewSettings> for VmfBlock {
    fn from(val: ViewSettings) -> Self {
        let mut key_values = IndexMap::new();
        key_values.insert("bSnapToGrid".to_string(), val.snap_to_grid.to_01_string());
        key_values.insert("bShowGrid".to_string(), val.show_grid.to_01_string());
        key_values.insert(
            "bShowLogicalGrid".to_string(),
            val.show_logical_grid.to_01_string(),
        );
        key_values.insert("nGridSpacing".to_string(), val.grid_spacing.to_string());
        key_values.insert("bShow3DGrid".to_string(), val.show_3d_grid.to_01_string());
        key_values.extend(val.extra.key_values);

        VmfBlock {
            name: "viewsettings".to_string(),
            key_values,
            blocks: val.extra.blocks,
        }
    }
}

impl VmfSerializable for ViewSettings {
    fn to_vmf_string(&self, indent_level: usize) -> String {
        let indent = "\t".repeat(indent_level);
        let mut output = String::with_capacity(64);

        output.push_str(&format!("{0}viewsettings\n{0}{{\n", indent));
        output.push_str(&format!(
            "{}\t\"bSnapToGrid\" \"{}\"\n",
            indent,
            self.snap_to_grid.to_01_string()
        ));
        output.push_str(&format!(
            "{}\t\"bShowGrid\" \"{}\"\n",
            indent,
            self.show_grid.to_01_string()
        ));
        output.push_str(&format!(
            "{}\t\"bShowLogicalGrid\" \"{}\"\n",
            indent,
            self.show_logical_grid.to_01_string()
        ));
        output.push_str(&format!(
            "{}\t\"nGridSpacing\" \"{}\"\n",
            indent, self.grid_spacing
        ));
        output.push_str(&format!(
            "{}\t\"bShow3DGrid\" \"{}\"\n",
            indent,
            self.show_3d_grid.to_01_string()
        ));
        self.extra.write_key_values(&mut output, indent_level + 1);
        self.extra.write_blocks(&mut output, indent_level + 1);

        output.push_str(&format!("{}}}\n", indent));
        output
    }
}
