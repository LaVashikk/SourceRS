//! This module provides structures for representing entities in a VMF file.

use crate::{
    VmfBlock, VmfSerializable,
    errors::{VmfError, VmfResult},
};
use derive_more::{Deref, DerefMut, IntoIterator};
use indexmap::IndexMap;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

use super::common::Editor;
use super::world::Solid;
use std::mem;

/// Represents an entity in a VMF file.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Entity {
    /// The key-value pairs associated with this entity.
    pub key_values: IndexMap<String, String>,
    /// The output connections of this entity.
    #[cfg_attr(
        feature = "serialization",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub connections: Option<Vec<(String, String)>>,
    /// The solids associated with this entity, if any.
    #[cfg_attr(
        feature = "serialization",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub solids: Option<Vec<Solid>>,
    /// The editor data for this entity.
    pub editor: Editor,
    /// Indicates if the entity is hidden within the editor.  This field
    /// is primarily used when parsing a `hidden` block within a VMF file,
    /// and is not serialized back when writing the VMF.
    #[cfg_attr(feature = "serialization", serde(default, skip_serializing))]
    pub is_hidden: bool,
}

impl Entity {
    /// Creates a new `Entity` with the specified classname and ID.
    ///
    /// # Arguments
    ///
    /// * `classname` - The classname of the entity (e.g., "func_detail", "info_player_start").
    /// * `id` - The unique ID of the entity.
    ///
    /// # Example
    ///
    /// ```
    /// use vmf_forge::prelude::*;
    ///
    /// let entity = Entity::new("info_player_start", 1);
    /// assert_eq!(entity.classname(), Some("info_player_start"));
    /// assert_eq!(entity.id(), 1);
    /// ```
    pub fn new(classname: impl Into<String>, id: u64) -> Self {
        let mut key_values = IndexMap::with_capacity(12);
        key_values.insert("classname".to_string(), classname.into());
        key_values.insert("id".to_string(), id.to_string());
        Entity {
            key_values,
            connections: None,
            solids: None,
            editor: Editor::default(),
            is_hidden: false,
        }
    }

    /// Sets a key-value pair for the entity.  If the key already exists,
    /// its value is updated.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to set.
    /// * `value` - The value to set for the key.
    pub fn set(&mut self, key: String, value: String) {
        self.key_values.insert(key, value);
    }

    /// Removes a key-value pair from the entity, preserving the order of other keys.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to remove.
    ///
    /// # Returns
    ///
    /// An `Option` containing the removed value, if the key was present.
    pub fn remove_key(&mut self, key: &str) -> Option<String> {
        self.key_values.shift_remove(key)
    }

    /// Removes a key-value pair from the entity, potentially changing the order of other keys.
    /// This is faster than `remove_key` but does not preserve insertion order.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to remove.
    ///
    /// # Returns
    ///
    /// An `Option` containing the removed value, if the key was present.
    pub fn swap_remove_key(&mut self, key: &str) -> Option<String> {
        self.key_values.swap_remove(key)
    }

    /// Gets the value associated with the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the value for.
    ///
    /// # Returns
    ///
    /// An `Option` containing a reference to the value, if the key is present.
    pub fn get(&self, key: &str) -> Option<&String> {
        self.key_values.get(key)
    }

    /// Gets a mutable reference to the value associated with the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to get the value for.
    ///
    /// # Returns
    ///
    /// An `Option` containing a mutable reference to the value, if the key is present.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut String> {
        self.key_values.get_mut(key)
    }

    /// Returns the classname of the entity.
    ///
    /// # Returns
    ///
    /// An `Option` containing the classname, if it exists.
    pub fn classname(&self) -> Option<&str> {
        self.key_values.get("classname").map(|s| s.as_str())
    }

    /// Returns the targetname of the entity.
    ///
    /// # Returns
    ///
    /// An `Option` containing the targetname, if it exists.
    pub fn targetname(&self) -> Option<&str> {
        self.key_values.get("targetname").map(|s| s.as_str())
    }

    /// Returns the ID of the entity.
    pub fn id(&self) -> u64 {
        self.key_values
            .get("id")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
    }

    /// Returns the model of the entity.
    ///
    /// # Returns
    ///
    /// An `Option` containing the model path, if it exists.
    pub fn model(&self) -> Option<&str> {
        self.key_values.get("model").map(|s| s.as_str())
    }

    /// Adds an output connection to the entity.
    ///
    /// # Arguments
    ///
    /// * `output` - The name of the output on this entity.
    /// * `target_entity` - The targetname of the entity to connect to.
    /// * `input` - The name of the input on the target entity.
    /// * `parms` - The parameters to pass to the input.
    /// * `delay` - The delay before the input is triggered, in seconds.
    /// * `fire_limit` - The number of times the output can be fired (-1 for unlimited).
    ///
    /// # Example
    ///
    /// ```
    /// use vmf_forge::prelude::*;
    ///
    /// let mut entity = Entity::new("logic_relay", 1);
    /// entity.add_connection("OnTrigger", "my_door", "Open", "", 0.0, -1);
    /// ```
    pub fn add_connection(
        &mut self,
        output: impl Into<String>,
        target_entity: impl AsRef<str>,
        input: impl AsRef<str>,
        parms: impl AsRef<str>,
        delay: f32,
        fire_limit: i32,
    ) {
        let input_result = format!(
            "{}\x1B{}\x1B{}\x1B{}\x1B{}",
            target_entity.as_ref(),
            input.as_ref(),
            parms.as_ref(),
            delay,
            fire_limit
        );
        if let Some(connections) = &mut self.connections {
            connections.push((output.into(), input_result));
        } else {
            self.connections = Some(vec![(output.into(), input_result)]);
        }
    }

    /// Removes all connections from this entity.
    pub fn clear_connections(&mut self) {
        self.connections = None;
    }

    /// Checks if a specific connection exists.
    ///
    /// # Arguments
    /// * `output` The output to check
    /// * `input` The input to check
    ///
    /// # Returns
    /// * `true` if the connection exists, `false` otherwise.
    pub fn has_connection(&self, output: &str, input: &str) -> bool {
        if let Some(connections) = &self.connections {
            connections.iter().any(|(o, i)| o == output && i == input)
        } else {
            false
        }
    }
}

impl TryFrom<VmfBlock> for Entity {
    type Error = VmfError;

    fn try_from(block: VmfBlock) -> VmfResult<Self> {
        // Extract key-value pairs from the block
        let key_values = block.key_values;

        // Searches for nested blocks and extracts the necessary information
        let mut ent = Self {
            key_values,
            ..Default::default()
        };
        let mut solids = Vec::with_capacity(block.blocks.len());

        for mut inner_block in block.blocks {
            match inner_block.name.as_str() {
                "editor" => ent.editor = Editor::try_from(inner_block)?,
                "connections" => ent.connections = process_connections(inner_block.key_values),
                "solid" => solids.push(Solid::try_from(inner_block)?),
                "hidden" => {
                    if !inner_block.blocks.is_empty() {
                        // Take ownership of the first block instead of cloning
                        let hidden_block = mem::take(&mut inner_block.blocks[0]);
                        solids.push(Solid::try_from(hidden_block)?)
                    }
                }
                _ => {
                    #[cfg(feature = "debug_assert_info")]
                    debug_assert!(
                        false,
                        "Unexpected block name: {}, id: {:?}",
                        inner_block.name,
                        ent.key_values.get("id")
                    );
                }
            }
        }

        if !solids.is_empty() {
            ent.solids = Some(solids);
        }

        Ok(ent)
    }
}

impl From<Entity> for VmfBlock {
    fn from(val: Entity) -> Self {
        let editor = val.editor.into();

        VmfBlock {
            name: "entity".to_string(),
            key_values: val.key_values,
            blocks: vec![editor],
        }
    }
}

impl VmfSerializable for Entity {
    fn to_vmf_string(&self, indent_level: usize) -> String {
        let indent = "\t".repeat(indent_level);
        let mut output = String::with_capacity(256);

        // Writes the main entity block
        output.push_str(&format!("{0}entity\n{0}{{\n", indent));

        // Adds key_values of the main block
        for (key, value) in &self.key_values {
            output.push_str(&format!("{}\t\"{}\" \"{}\"\n", indent, key, value));
        }

        // Adds connections block
        if let Some(connections) = &self.connections {
            output.push_str(&format!("{0}\tconnections\n{0}\t{{\n", indent));
            for (out, inp) in connections {
                output.push_str(&format!("{}\t\t\"{}\" \"{}\"\n", indent, out, inp));
            }
            output.push_str(&format!("{}\t}}\n", indent));
        }

        // Solids block
        if let Some(solids) = &self.solids {
            for solid in solids {
                output.push_str(&solid.to_vmf_string(indent_level + 1));
            }
        }

        // Editor block
        output.push_str(&self.editor.to_vmf_string(indent_level + 1));

        output.push_str(&format!("{}}}\n", indent));

        output
    }
}

/// A collection of entities.
#[derive(Debug, Default, Clone, Deref, DerefMut, IntoIterator, PartialEq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Entities(pub Vec<Entity>);

impl Entities {
    /// Returns an iterator over the entities that have the specified key-value pair.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to search for.
    /// * `value` - The value to search for.
    pub fn find_by_keyvalue<'a>(
        &'a self,
        key: &'a str,
        value: &'a str,
    ) -> impl Iterator<Item = &'a Entity> + 'a {
        self.iter()
            .filter(move |ent| ent.key_values.get(key).is_some_and(|v| v == value))
    }

    /// Returns an iterator over the entities that have the specified key-value pair, allowing modification.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to search for.
    /// * `value` - The value to search for.
    pub fn find_by_keyvalue_mut<'a>(
        &'a mut self,
        key: &'a str,
        value: &'a str,
    ) -> impl Iterator<Item = &'a mut Entity> + 'a {
        self.iter_mut()
            .filter(move |ent| ent.key_values.get(key).is_some_and(|v| v == value))
    }

    /// Returns an iterator over the entities with the specified classname.
    ///
    /// # Arguments
    ///
    /// * `classname` - The classname to search for.
    pub fn find_by_classname<'a>(
        &'a self,
        classname: &'a str,
    ) -> impl Iterator<Item = &'a Entity> + 'a {
        self.find_by_keyvalue("classname", classname)
    }

    /// Returns an iterator over the entities with the specified targetname.
    ///
    /// # Arguments
    ///
    /// * `name` - The targetname to search for.
    pub fn find_by_name<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Entity> + 'a {
        self.find_by_keyvalue("targetname", name)
    }

    /// Returns an iterator over the entities with the specified classname, allowing modification.
    ///
    /// # Arguments
    ///
    /// * `classname` - The classname to search for.
    pub fn find_by_classname_mut<'a>(
        &'a mut self,
        classname: &'a str,
    ) -> impl Iterator<Item = &'a mut Entity> + 'a {
        self.find_by_keyvalue_mut("classname", classname)
    }

    /// Returns an iterator over the entities with the specified targetname, allowing modification.
    ///
    /// # Arguments
    ///
    /// * `name` - The targetname to search for.
    pub fn find_by_name_mut<'a>(
        &'a mut self,
        name: &'a str,
    ) -> impl Iterator<Item = &'a mut Entity> + 'a {
        self.find_by_keyvalue_mut("targetname", name)
    }

    /// Returns an iterator over the entities with the specified model.
    ///
    /// # Arguments
    ///
    /// * `model` - The model to search for.
    pub fn find_by_model<'a>(&'a self, model: &'a str) -> impl Iterator<Item = &'a Entity> + 'a {
        self.find_by_keyvalue("model", model)
    }

    /// Returns an iterator over the entities with the specified model, allowing modification.
    ///
    /// # Arguments
    ///
    /// * `model` - The model to search for.
    pub fn find_by_model_mut<'a>(
        &'a mut self,
        model: &'a str,
    ) -> impl Iterator<Item = &'a mut Entity> + 'a {
        self.find_by_keyvalue_mut("model", model)
    }

    /// Removes an entity by its ID.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The ID of the entity to remove.
    ///
    /// # Returns
    ///
    /// An `Option` containing the removed `Entity`, if found. Returns `None`
    /// if no entity with the given ID exists.
    pub fn remove_entity(&mut self, entity_id: i32) -> Option<Entity> {
        self
            .iter()
            .position(|e| e.key_values.get("id") == Some(&entity_id.to_string()))
            .map(|index| self.remove(index))
    }

    /// Removes all entities that have a matching key-value pair.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to check.
    /// * `value` - The value to compare against.
    pub fn remove_by_keyvalue(&mut self, key: &str, value: &str) {
        self.retain(|ent| ent.key_values.get(key).map(|v| v != value).unwrap_or(true));
    }
}

// utils func
fn process_connections(map: IndexMap<String, String>) -> Option<Vec<(String, String)>> {
    if map.is_empty() {
        return None;
    }

    // Estimate: we assume an average of no more than 2 records per output
    let mut result = Vec::with_capacity(map.len() * 2);
    for (key, value) in map.iter() {
        // Используем iter, т.к. map по ссылке
        for part in value.split('\r') {
            result.push((key.clone(), part.to_string()));
        }
    }

    Some(result)
}
