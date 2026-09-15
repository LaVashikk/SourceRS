//! This module provides common structures and functions used across the VMF parser.

use indexmap::IndexMap;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

use crate::parse_ctx::{FromBlock, ParseCtx, impl_try_from_block};
use crate::utils::{take_key, take_multi_parse, take_opt_parse};
use crate::{Extras, VmfBlock, VmfSerializable, join_repeated_key, utils::To01String};

/// Represents the editor data of a VMF entity or solid.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Editor {
    /// The color of the entity in the editor, in "R G B" format.
    pub color: String,
    /// The IDs of the visgroups this entity is in, if any.
    #[cfg_attr(
        feature = "serialization",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub visgroup_ids: Vec<i32>,
    /// The ID of the group this entity is in, if any.
    #[cfg_attr(
        feature = "serialization",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub group_id: Option<i32>,
    /// Whether the entity is shown in the visgroup.
    pub visgroup_shown: bool,
    /// Whether the entity should automatically be shown in the visgroup.
    pub visgroup_auto_shown: bool,
    /// Comments associated with the entity, if any.
    #[cfg_attr(
        feature = "serialization",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub comments: Option<String>,
    /// The logical position of the entity in the editor, in "[x y]" format.
    #[cfg_attr(
        feature = "serialization",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub logical_pos: Option<String>,
    /// Keys and blocks this struct does not model.
    #[cfg_attr(feature = "serialization", serde(default, skip_serializing_if = "Extras::is_empty"))]
    pub extra: Extras,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            color: "255 255 255".to_string(),
            visgroup_ids: Vec::new(),
            group_id: None,
            visgroup_shown: true,
            visgroup_auto_shown: true,
            comments: None,
            logical_pos: None,
            extra: Extras::default(),
        }
    }
}

impl Editor {
    /// Puts the object into a visgroup. Returns false if it was already in it.
    pub fn join_visgroup(&mut self, id: i32) -> bool {
        if self.visgroup_ids.contains(&id) {
            return false;
        }
        self.visgroup_ids.push(id);
        true
    }

    /// Takes the object out of a visgroup. Returns false if it was not in it.
    pub fn leave_visgroup(&mut self, id: i32) -> bool {
        let before = self.visgroup_ids.len();
        self.visgroup_ids.retain(|&member| member != id);
        before != self.visgroup_ids.len()
    }
}

impl FromBlock for Editor {
    fn from_block(mut block: VmfBlock, ctx: &mut ParseCtx) -> Self {
        let blocks = std::mem::take(&mut block.blocks);
        let kv = &mut block.key_values;

        Self {
            color: take_key(kv, "color").unwrap_or_else(|| "255 255 255".to_string()),
            visgroup_ids: take_multi_parse::<i32>(kv, "visgroupid", ctx),
            group_id: take_opt_parse::<i32>(kv, "groupid", ctx),
            visgroup_shown: take_key(kv, "visgroupshown").is_some_and(|v| v == "1"),
            visgroup_auto_shown: take_key(kv, "visgroupautoshown").is_some_and(|v| v == "1"),
            comments: take_key(kv, "comments"),
            logical_pos: take_key(kv, "logicalpos"),
            extra: Extras {
                key_values: std::mem::take(kv),
                blocks,
            },
        }
    }
}

impl_try_from_block!(Editor);

impl From<Editor> for VmfBlock {
    fn from(val: Editor) -> VmfBlock {
        let mut key_values = IndexMap::new();
        key_values.insert("color".to_string(), val.color);
        if !val.visgroup_ids.is_empty() {
            key_values.insert("visgroupid".to_string(), join_repeated_key(val.visgroup_ids));
        }
        if let Some(group_id) = val.group_id {
            key_values.insert("groupid".to_string(), group_id.to_string());
        }
        key_values.insert(
            "visgroupshown".to_string(),
            val.visgroup_shown.to_01_string(),
        );
        key_values.insert(
            "visgroupautoshown".to_string(),
            val.visgroup_auto_shown.to_01_string(),
        );
        if let Some(comments) = val.comments {
            key_values.insert("comments".to_string(), comments);
        }
        if let Some(logical_pos) = val.logical_pos {
            key_values.insert("logicalpos".to_string(), logical_pos);
        }
        key_values.extend(val.extra.key_values);

        VmfBlock {
            name: "editor".to_string(),
            key_values,
            blocks: val.extra.blocks,
        }
    }
}

impl VmfSerializable for Editor {
    fn to_vmf_string(&self, indent_level: usize) -> String {
        let indent = "\t".repeat(indent_level);
        let mut output = String::with_capacity(128);

        output.push_str(&format!("{0}editor\n{0}{{\n", indent));
        output.push_str(&format!("{}\t\"color\" \"{}\"\n", indent, self.color));
        for visgroup_id in &self.visgroup_ids {
            output.push_str(&format!("{}\t\"visgroupid\" \"{}\"\n", indent, visgroup_id));
        }
        if let Some(group_id) = self.group_id {
            output.push_str(&format!("{}\t\"groupid\" \"{}\"\n", indent, group_id));
        }
        output.push_str(&format!(
            "{}\t\"visgroupshown\" \"{}\"\n",
            indent,
            self.visgroup_shown.to_01_string()
        ));
        output.push_str(&format!(
            "{}\t\"visgroupautoshown\" \"{}\"\n",
            indent,
            self.visgroup_auto_shown.to_01_string()
        ));
        if let Some(comments) = &self.comments {
            output.push_str(&format!("{}\t\"comments\" \"{}\"\n", indent, comments));
        }
        if let Some(logical_pos) = &self.logical_pos {
            output.push_str(&format!("{}\t\"logicalpos\" \"{}\"\n", indent, logical_pos));
        }
        self.extra.write_key_values(&mut output, indent_level + 1);
        self.extra.write_blocks(&mut output, indent_level + 1);

        output.push_str(&format!("{}}}\n", indent));
        output
    }
}
