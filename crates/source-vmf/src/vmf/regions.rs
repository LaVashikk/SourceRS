//! This module provides structures for representing region-specific data in a VMF file, such as cameras and cordons.

use derive_more::{Deref, DerefMut};
use indexmap::IndexMap;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

use crate::parse_ctx::{FromBlock, ParseCtx, impl_try_from_block};
use crate::utils::{To01String, take_bool, take_key, take_parse, take_str};
use crate::{Extras, VmfBlock, VmfSerializable};

impl_try_from_block!(Cameras, Camera, Cordons, Cordon);

/// Represents the camera data in a VMF file.
#[derive(Debug, Default, Clone, PartialEq, Deref, DerefMut)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Cameras {
    /// The index of the active camera.
    pub active: i8,
    /// The list of cameras.
    #[deref]
    #[deref_mut]
    pub cams: Vec<Camera>,
    /// Keys and blocks this struct does not model.
    #[cfg_attr(feature = "serialization", serde(default, skip_serializing_if = "Extras::is_empty"))]
    pub extra: Extras,
}

impl FromBlock for Cameras {
    fn from_block(mut block: VmfBlock, ctx: &mut ParseCtx) -> Self {
        ctx.enter("cameras");
        let mut cams = Vec::with_capacity(block.blocks.len());
        let mut unknown = Vec::new();

        for (index, group) in block.blocks.drain(..).enumerate() {
            if group.name.eq_ignore_ascii_case("camera") {
                ctx.enter_indexed("camera", index);
                cams.push(Camera::from_block(group, ctx));
                ctx.leave();
            } else {
                ctx.warn(format!("unexpected block '{}', kept as-is", group.name));
                unknown.push(group);
            }
        }

        let kv = &mut block.key_values;
        let cameras = Self {
            active: take_parse(kv, "activecamera", -1, ctx),
            cams,
            extra: Extras {
                key_values: std::mem::take(kv),
                blocks: unknown,
            },
        };
        ctx.leave();
        cameras
    }
}

impl From<Cameras> for VmfBlock {
    fn from(val: Cameras) -> Self {
        let mut blocks = Vec::with_capacity(val.cams.len());

        for cam in val.cams {
            blocks.push(cam.into());
        }

        blocks.extend(val.extra.blocks);

        let mut key_values = IndexMap::new();
        key_values.insert("activecamera".to_string(), val.active.to_string());
        key_values.extend(val.extra.key_values);

        VmfBlock {
            name: "cameras".to_string(),
            key_values,
            blocks,
        }
    }
}

impl VmfSerializable for Cameras {
    fn to_vmf_string(&self, indent_level: usize) -> String {
        let indent: String = "\t".repeat(indent_level);
        let mut output = String::with_capacity(64);

        output.push_str(&format!("{0}cameras\n{0}{{\n", indent));
        output.push_str(&format!(
            "{}\t\"activecamera\" \"{}\"\n",
            indent, self.active
        ));
        self.extra.write_key_values(&mut output, indent_level + 1);

        for cam in &self.cams {
            output.push_str(&format!("{0}\tcamera\n{0}\t{{\n", indent));
            output.push_str(&format!(
                "{}\t\t\"position\" \"{}\"\n",
                indent, cam.position
            ));
            output.push_str(&format!("{}\t\t\"look\" \"{}\"\n", indent, cam.look));
            cam.extra.write_key_values(&mut output, indent_level + 2);
            cam.extra.write_blocks(&mut output, indent_level + 2);
            output.push_str(&format!("{}\t}}\n", indent));
        }
        self.extra.write_blocks(&mut output, indent_level + 1);

        output.push_str(&format!("{}}}\n", indent));
        output
    }
}

/// Represents a single camera in a VMF file.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Camera {
    /// The position of the camera in the VMF coordinate system.
    pub position: String, // vertex
    /// The point at which the camera is looking, in the VMF coordinate system.
    pub look: String, // vertex
    /// Keys and blocks this struct does not model.
    #[cfg_attr(feature = "serialization", serde(default, skip_serializing_if = "Extras::is_empty"))]
    pub extra: Extras,
}

impl FromBlock for Camera {
    fn from_block(mut block: VmfBlock, ctx: &mut ParseCtx) -> Self {
        let blocks = std::mem::take(&mut block.blocks);
        let kv = &mut block.key_values;
        Self {
            position: take_str(kv, "position", "0 0 0", ctx),
            look: take_str(kv, "look", "0 0 0", ctx),
            extra: Extras {
                key_values: std::mem::take(kv),
                blocks,
            },
        }
    }
}

impl From<Camera> for VmfBlock {
    fn from(val: Camera) -> Self {
        let mut key_values = IndexMap::new();
        key_values.insert("position".to_string(), val.position);
        key_values.insert("look".to_string(), val.look);
        key_values.extend(val.extra.key_values);

        VmfBlock {
            name: "camera".to_string(),
            key_values,
            blocks: val.extra.blocks,
        }
    }
}

/// Represents the cordons data in a VMF file.
#[derive(Debug, Default, Clone, PartialEq, Deref, DerefMut)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Cordons {
    /// The index of the active cordon.
    pub active: i8,
    /// The list of cordons.
    #[deref]
    #[deref_mut]
    pub cordons: Vec<Cordon>,
    /// Keys and blocks this struct does not model.
    #[cfg_attr(feature = "serialization", serde(default, skip_serializing_if = "Extras::is_empty"))]
    pub extra: Extras,
}

impl FromBlock for Cordons {
    fn from_block(mut block: VmfBlock, ctx: &mut ParseCtx) -> Self {
        ctx.enter("cordons");
        let mut cordons = Vec::with_capacity(block.blocks.len());
        let mut unknown = Vec::new();

        for (index, group) in block.blocks.drain(..).enumerate() {
            if group.name.eq_ignore_ascii_case("cordon") {
                ctx.enter_indexed("cordon", index);
                cordons.push(Cordon::from_block(group, ctx));
                ctx.leave();
            } else {
                ctx.warn(format!("unexpected block '{}', kept as-is", group.name));
                unknown.push(group);
            }
        }

        let kv = &mut block.key_values;
        let result = Self {
            active: take_parse(kv, "active", 0, ctx),
            cordons,
            extra: Extras {
                key_values: std::mem::take(kv),
                blocks: unknown,
            },
        };
        ctx.leave();
        result
    }
}

impl From<Cordons> for VmfBlock {
    fn from(val: Cordons) -> Self {
        let mut blocks = Vec::new();

        // Converts each  Cordon to a VmfBlock and adds it to the `blocks` vector
        for cordon in val.cordons {
            blocks.push(cordon.into());
        }
        blocks.extend(val.extra.blocks);

        // Creates a VmfBlock for Cordons
        let mut key_values = IndexMap::new();
        key_values.insert("active".to_string(), val.active.to_string());
        key_values.extend(val.extra.key_values);

        VmfBlock {
            name: "cordons".to_string(),
            key_values,
            blocks,
        }
    }
}

impl VmfSerializable for Cordons {
    fn to_vmf_string(&self, indent_level: usize) -> String {
        let indent = "\t".repeat(indent_level);
        let mut output = String::with_capacity(256);

        // Start of Cordons block
        output.push_str(&format!("{0}cordons\n{0}{{\n", indent));
        output.push_str(&format!("{}\t\"active\" \"{}\"\n", indent, self.active));
        self.extra.write_key_values(&mut output, indent_level + 1);

        // Iterates through all Cordons and adds their string representation
        for cordon in &self.cordons {
            output.push_str(&cordon.to_vmf_string(indent_level + 1));
        }
        self.extra.write_blocks(&mut output, indent_level + 1);

        output.push_str(&format!("{}}}\n", indent));

        output
    }
}

/// Represents a single cordon in a VMF file.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Cordon {
    /// The name of the cordon.
    pub name: String,
    /// Whether the cordon is active.
    pub active: bool,
    /// The minimum point of the cordon's bounding box.
    pub min: String, // vertex
    /// The maximum point of the cordon's bounding box.
    pub max: String, // vertex
    /// Keys and blocks this struct does not model.
    #[cfg_attr(feature = "serialization", serde(default, skip_serializing_if = "Extras::is_empty"))]
    pub extra: Extras,
}

impl FromBlock for Cordon {
    fn from_block(mut block: VmfBlock, ctx: &mut ParseCtx) -> Self {
        // Mins/maxs reside in a 'box' sub-block in newer VMFs, or directly on cordon in older ones.
        let from_box = block.blocks.first_mut().and_then(|sub_block| {
            let min = take_key(&mut sub_block.key_values, "mins");
            let max = take_key(&mut sub_block.key_values, "maxs");
            min.zip(max)
        });

        // Drop empty 'box' block so it is not preserved in extras.
        block
            .blocks
            .retain(|b| !(b.name.eq_ignore_ascii_case("box") && b.key_values.is_empty()));
        let blocks = std::mem::take(&mut block.blocks);

        let kv = &mut block.key_values;
        let (min, max) = match from_box {
            Some(pair) => pair,
            None => (
                take_str(kv, "mins", "0 0 0", ctx),
                take_str(kv, "maxs", "0 0 0", ctx),
            ),
        };

        Self {
            name: take_str(kv, "name", "cordon", ctx),
            active: take_bool(kv, "active", false, ctx),
            min,
            max,
            extra: Extras {
                key_values: std::mem::take(kv),
                blocks,
            },
        }
    }
}

impl From<Cordon> for VmfBlock {
    fn from(val: Cordon) -> Self {
        // Creates key_values for Cordon
        let mut key_values = IndexMap::new();
        key_values.insert("name".to_string(), val.name);
        key_values.insert("active".to_string(), val.active.to_01_string());
        key_values.extend(val.extra.key_values);

        // Creates a block for the box with `mins/maxs`
        let mut box_block_key_values = IndexMap::new();
        box_block_key_values.insert("mins".to_string(), val.min);
        box_block_key_values.insert("maxs".to_string(), val.max);

        // Creates a VmfBlock for the box
        let box_block = VmfBlock {
            name: "box".to_string(),
            key_values: box_block_key_values,
            blocks: vec![],
        };

        // Creates the main VmfBlock for Cordon
        let mut blocks = vec![box_block];
        blocks.extend(val.extra.blocks);

        VmfBlock {
            name: "cordon".to_string(),
            key_values,
            blocks,
        }
    }
}

impl VmfSerializable for Cordon {
    fn to_vmf_string(&self, indent_level: usize) -> String {
        let indent: String = "\t".repeat(indent_level);
        let mut output = String::with_capacity(64);

        // Start of Cordon block
        output.push_str(&format!("{0}cordon\n{0}{{\n", indent));
        output.push_str(&format!("{}\t\"name\" \"{}\"\n", indent, self.name));
        output.push_str(&format!(
            "{}\t\"active\" \"{}\"\n",
            indent,
            self.active.to_01_string()
        ));
        self.extra.write_key_values(&mut output, indent_level + 1);

        // Adds a nested block with coordinates
        output.push_str(&format!("{0}\tbox\n{0}\t{{\n", indent));
        output.push_str(&format!("{}\t\t\"mins\" \"{}\"\n", indent, self.min));
        output.push_str(&format!("{}\t\t\"maxs\" \"{}\"\n", indent, self.max));
        output.push_str(&format!("{}\t}}\n", indent)); // end of `box``
        self.extra.write_blocks(&mut output, indent_level + 1);

        // End of Cordon block
        output.push_str(&format!("{}}}\n", indent));

        output
    }
}
