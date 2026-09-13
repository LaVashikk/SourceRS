//! This module provides common structures and functions used across the VMF parser.

use indexmap::IndexMap;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

use crate::utils::{get_key_ref, take_and_parse_key, take_key_or_default};
use crate::{
    VmfBlock, VmfSerializable,
    errors::{VmfError, VmfResult},
    utils::To01String,
};

/// Represents the editor data of a VMF entity or solid.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Editor {
    /// The color of the entity in the editor, in "R G B" format.
    pub color: String,
    /// The ID of the visgroup this entity is in, if any.
    #[cfg_attr(
        feature = "serialization",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub visgroup_id: Option<i32>,
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
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            color: "255 255 255".to_string(),
            visgroup_id: None,
            group_id: None,
            visgroup_shown: true,
            visgroup_auto_shown: true,
            comments: None,
            logical_pos: None,
        }
    }
}

impl TryFrom<VmfBlock> for Editor {
    type Error = VmfError;

    fn try_from(mut block: VmfBlock) -> VmfResult<Self> {
        let kv = &mut block.key_values;

        Ok(Self {
            color: take_key_or_default(kv, "color", "255 255 255".to_string()),
            visgroup_id: take_and_parse_key::<i32>(kv, "visgroupid").ok(),
            group_id: take_and_parse_key::<i32>(kv, "groupid").ok(),
            visgroup_shown: get_key_ref(kv, "visgroupshown").is_ok_and(|v| v == "1"),
            visgroup_auto_shown: get_key_ref(kv, "visgroupautoshown").is_ok_and(|v| v == "1"),
            comments: kv.swap_remove("comments"),
            logical_pos: kv.swap_remove("logicalpos"),
        })
    }
}

impl From<Editor> for VmfBlock {
    fn from(val: Editor) -> VmfBlock {
        let mut key_values = IndexMap::new();
        key_values.insert("color".to_string(), val.color);
        if let Some(visgroup_id) = val.visgroup_id {
            key_values.insert("visgroupid".to_string(), visgroup_id.to_string());
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

        VmfBlock {
            name: "editor".to_string(),
            key_values,
            blocks: Vec::new(),
        }
    }
}

impl VmfSerializable for Editor {
    fn to_vmf_string(&self, indent_level: usize) -> String {
        let indent = "\t".repeat(indent_level);
        let mut output = String::with_capacity(128);

        output.push_str(&format!("{0}editor\n{0}{{\n", indent));
        output.push_str(&format!("{}\t\"color\" \"{}\"\n", indent, self.color));
        if let Some(visgroup_id) = self.visgroup_id {
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

        output.push_str(&format!("{}}}\n", indent));
        output
    }
}
