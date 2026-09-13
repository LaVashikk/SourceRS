//! This module provides structures for representing region-specific data in a VMF file, such as cameras and cordons.

use derive_more::{Deref, DerefMut};
use indexmap::IndexMap;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

use crate::utils::{To01String, get_key_ref, take_and_parse_key, take_key_owned};
use crate::{
    VmfBlock, VmfSerializable,
    errors::{VmfError, VmfResult},
};

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
}

impl TryFrom<VmfBlock> for Cameras {
    type Error = VmfError;

    fn try_from(mut block: VmfBlock) -> VmfResult<Self> {
        let mut cams = Vec::with_capacity(block.blocks.len());
        for group in block.blocks {
            cams.push(Camera::try_from(group)?);
        }

        Ok(Self {
            active: take_and_parse_key::<i8>(&mut block.key_values, "activecamera")?,
            cams,
        })
    }
}

impl From<Cameras> for VmfBlock {
    fn from(val: Cameras) -> Self {
        let mut blocks = Vec::with_capacity(val.cams.len());

        for cam in val.cams {
            blocks.push(cam.into());
        }

        let mut key_values = IndexMap::new();
        key_values.insert("active".to_string(), val.active.to_string());

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

        for cam in &self.cams {
            output.push_str(&format!("{0}\tcamera\n{0}\t{{\n", indent));
            output.push_str(&format!(
                "{}\t\t\"position\" \"{}\"\n",
                indent, cam.position
            ));
            output.push_str(&format!("{}\t\t\"look\" \"{}\"\n", indent, cam.look));
            output.push_str(&format!("{}\t}}\n", indent));
        }

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
}

impl TryFrom<VmfBlock> for Camera {
    type Error = VmfError;

    fn try_from(mut block: VmfBlock) -> VmfResult<Self> {
        let kv = &mut block.key_values;
        Ok(Self {
            position: take_key_owned(kv, "position")?,
            look: take_key_owned(kv, "look")?,
        })
    }
}

impl From<Camera> for VmfBlock {
    fn from(val: Camera) -> Self {
        let mut key_values = IndexMap::new();
        key_values.insert("position".to_string(), val.position);
        key_values.insert("look".to_string(), val.look);

        VmfBlock {
            name: "camera".to_string(),
            key_values,
            ..Default::default()
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
}

impl TryFrom<VmfBlock> for Cordons {
    type Error = VmfError;

    fn try_from(mut block: VmfBlock) -> VmfResult<Self> {
        let mut cordons = Vec::with_capacity(block.blocks.len());
        for group in block.blocks {
            cordons.push(Cordon::try_from(group)?);
        }

        Ok(Self {
            active: take_and_parse_key::<i8>(&mut block.key_values, "active")?,
            cordons,
        })
    }
}

impl From<Cordons> for VmfBlock {
    fn from(val: Cordons) -> Self {
        let mut blocks = Vec::new();

        // Converts each  Cordon to a VmfBlock and adds it to the `blocks` vector
        for cordon in val.cordons {
            blocks.push(cordon.into());
        }

        // Creates a VmfBlock for Cordons
        let mut key_values = IndexMap::new();
        key_values.insert("active".to_string(), val.active.to_string());

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

        // Iterates through all Cordons and adds their string representation
        for cordon in &self.cordons {
            output.push_str(&cordon.to_vmf_string(indent_level + 1));
        }

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
}

impl TryFrom<VmfBlock> for Cordon {
    type Error = VmfError;

    fn try_from(mut block: VmfBlock) -> VmfResult<Self> {
        // Attempt #1: Try to take ownership of "mins" and "maxs" from the first sub-block.
        let sub_block_result: Option<(String, String)> =
            block.blocks.get_mut(0).and_then(|sub_block| {
                // Try removing both keys using swap_remove (O(1)) and zip the Options.
                // zip returns Some only if *both* removals were successful.
                let maybe_min = sub_block.key_values.swap_remove("mins");
                let maybe_max = sub_block.key_values.swap_remove("maxs");
                maybe_min.zip(maybe_max)
            });

        // Decide where the final values come from
        let (min_string, max_string) = match sub_block_result {
            // Case 1: Successfully got both from the sub-block
            Some((min_val, max_val)) => Ok((min_val, max_val)),
            // Case 2: Failed to get both from sub-block (it didn't exist, or lacked one/both keys)
            None => {
                // Attempt #2: Take ownership from the parent block's key_values
                let min_res = take_key_owned(&mut block.key_values, "mins").map_err(|_| {
                    VmfError::InvalidFormat(
                        "Missing 'mins' key in Cordon block or its 'box' sub-block".to_string(),
                    )
                });
                let max_res = take_key_owned(&mut block.key_values, "maxs").map_err(|_| {
                    VmfError::InvalidFormat(
                        "Missing 'maxs' key in Cordon block or its 'box' sub-block".to_string(),
                    )
                });

                // Combine results, returning the first error encountered if any
                match (min_res, max_res) {
                    (Ok(min), Ok(max)) => Ok((min, max)),
                    (Err(e), _) => Err(e),
                    (_, Err(e)) => Err(e),
                }
            }
        }?;

        // Take ownership of 'name' and check 'active' from the parent block
        let name = take_key_owned(&mut block.key_values, "name")?;
        let active = get_key_ref(&block.key_values, "active")? == "1";

        Ok(Self {
            name,
            active,
            min: min_string,
            max: max_string,
        })
    }
}

impl From<Cordon> for VmfBlock {
    fn from(val: Cordon) -> Self {
        // Creates key_values for Cordon
        let mut key_values = IndexMap::new();
        key_values.insert("name".to_string(), val.name);
        key_values.insert("active".to_string(), val.active.to_01_string());

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
        VmfBlock {
            name: "cordon".to_string(),
            key_values,
            blocks: vec![box_block],
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

        // Adds a nested block with coordinates
        output.push_str(&format!("{0}\tbox\n{}\t{{\n", indent));
        output.push_str(&format!("{}\t\t\"mins\" \"{}\"\n", indent, self.min));
        output.push_str(&format!("{}\t\t\"maxs\" \"{}\"\n", indent, self.max));
        output.push_str(&format!("{}\t}}\n", indent)); // end of `box``

        // End of Cordon block
        output.push_str(&format!("{}}}\n", indent));

        output
    }
}
