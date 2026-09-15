//! This module provides the VMF parser implementation using the `source-kv` library.

use indexmap::IndexMap;

use crate::errors::VmfResult;
use crate::parse_ctx::{FromBlock, ParseCtx};
use crate::VmfBlock;
use crate::prelude::{Cameras, Entity, VersionInfo, ViewSettings, VisGroups, VmfFile, World};
use crate::vmf::regions::{Cordon, Cordons};

/// Parses a VMF string into a `VmfFile` struct.
///
/// # Arguments
///
/// * `input` - The VMF string to parse.
///
/// # Returns
///
/// A `VmfResult` containing the parsed `VmfFile` or a `VmfError` if parsing fails.
pub fn parse_vmf(input: &str) -> VmfResult<VmfFile> {
    let mut deserializer = source_kv::de::Deserializer::from_str(input);
    let root = deserializer.parse_root()?;

    let mut vmf_file = VmfFile::default();
    let mut ctx = ParseCtx::default();

    if let source_kv::de::Value::Obj(map) = root {
        for (name, values) in map {
            for value in values {
                let block = value_to_vmf_block(name.clone(), value);
                // Block names are case-insensitive, like every other KeyValues name.
                let kind = block.name.to_ascii_lowercase();
                match kind.as_str() {
                    // -- metadatas
                    "versioninfo" => vmf_file.versioninfo = VersionInfo::from_block(block, &mut ctx),
                    "visgroups" => vmf_file.visgroups = VisGroups::from_block(block, &mut ctx),
                    "viewsettings" => vmf_file.viewsettings = ViewSettings::from_block(block, &mut ctx),

                    // world
                    "world" => vmf_file.world = World::from_block(block, &mut ctx),

                    // -- entities
                    "entity" => {
                        ctx.enter_indexed("entity", vmf_file.entities.len());
                        let ent = Entity::from_block(block, &mut ctx);
                        ctx.leave();
                        vmf_file.entities.push(ent);
                    }
                    "hidden" => {
                        // Read all entities enclosed in the hidden block
                        for inner in block.blocks {
                            ctx.enter_indexed("hidden", vmf_file.hiddens.len());
                            let mut ent = Entity::from_block(inner, &mut ctx);
                            ctx.leave();
                            ent.is_hidden = true;
                            vmf_file.hiddens.push(ent)
                        }
                    }

                    // -- regions
                    "cameras" => vmf_file.cameras = Cameras::from_block(block, &mut ctx),
                    "cordons" => vmf_file.cordons = Cordons::from_block(block, &mut ctx),
                    // for old version of VMF
                    "cordon" => {
                        ctx.enter("cordon");
                        let cordon = Cordon::from_block(block, &mut ctx);
                        ctx.leave();
                        vmf_file.cordons.push(cordon);
                    }
                    _ => {
                        ctx.warn(format!("unexpected root block '{}', kept as-is", block.name));
                        vmf_file.extra_blocks.push(block);
                    }
                }
            }
        }
    }

    vmf_file.warnings = ctx.warnings;
    vmf_file.suppressed_warnings = ctx.suppressed;
    Ok(vmf_file)
}

/// Converts a `source_kv::de::Value` into a `VmfBlock`.
fn value_to_vmf_block(name: String, value: source_kv::de::Value) -> VmfBlock {
    match value {
        source_kv::de::Value::Str(s) => {
            let mut key_values = IndexMap::with_capacity(1);
            key_values.insert(name.clone(), s);
            VmfBlock {
                name,
                key_values,
                blocks: Vec::new(),
            }
        }
        source_kv::de::Value::Obj(map) => {
            let mut key_values = IndexMap::with_capacity(map.len());
            let mut blocks = Vec::with_capacity(map.len() / 4); // heuristics

            for (key, mut values) in map {
                if values.len() == 1 {
                    let val = values.pop().unwrap();
                    match val {
                        source_kv::de::Value::Str(s) => {
                            key_values.insert(key, s);
                        }
                        source_kv::de::Value::Obj(_) => {
                            blocks.push(value_to_vmf_block(key, val));
                        }
                    }
                } else {
                    // Repeated keys: string values are joined with '\r', while blocks become separate children
                    for val in values {
                        match val {
                            source_kv::de::Value::Str(s) => {
                                key_values
                                    .entry(key.clone())
                                    .and_modify(|existing: &mut String| {
                                        existing.push('\r');
                                        existing.push_str(&s);
                                    })
                                    .or_insert(s);
                            }
                            source_kv::de::Value::Obj(_) => {
                                blocks.push(value_to_vmf_block(key.clone(), val));
                            }
                        }
                    }
                }
            }
            VmfBlock {
                name,
                key_values,
                blocks,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn parse_block_valid_block() {
        let input = "entity { \"classname\" \"logic_relay\" }";
        let mut deserializer = source_kv::de::Deserializer::from_str(input);
        let root = deserializer.parse_root().unwrap();
        
        if let source_kv::de::Value::Obj(map) = root {
            let (name, mut values) = map.into_iter().next().unwrap();
            let block = value_to_vmf_block(name, values.remove(0));
            
            assert_eq!(block.name, "entity");
            assert_eq!(
                block.key_values.get("classname"),
                Some(&"logic_relay".to_string())
            );
            assert!(block.blocks.is_empty());
        } else {
            panic!("Expected Obj");
        }
    }

    #[test]
    fn parse_block_nested_blocks() {
        let input = "entity { \"classname\" \"logic_relay\" solid { \"id\" \"1\" } }";
        let mut deserializer = source_kv::de::Deserializer::from_str(input);
        let root = deserializer.parse_root().unwrap();

        if let source_kv::de::Value::Obj(map) = root {
            let (name, mut values) = map.into_iter().next().unwrap();
            let block = value_to_vmf_block(name, values.remove(0));

            assert_eq!(block.name, "entity");
            assert_eq!(
                block.key_values.get("classname"),
                Some(&"logic_relay".to_string())
            );
            assert_eq!(block.blocks.len(), 1);
            assert_eq!(block.blocks[0].name, "solid");
            assert_eq!(block.blocks[0].key_values.get("id"), Some(&"1".to_string()));
        } else {
            panic!("Expected Obj");
        }
    }

    #[test]
    fn parse_block_empty_block() {
        let input = "entity { }";
        let mut deserializer = source_kv::de::Deserializer::from_str(input);
        let root = deserializer.parse_root().unwrap();

        if let source_kv::de::Value::Obj(map) = root {
            let (name, mut values) = map.into_iter().next().unwrap();
            let block = value_to_vmf_block(name, values.remove(0));

            assert_eq!(block.name, "entity");
            assert!(block.key_values.is_empty());
            assert!(block.blocks.is_empty());
        } else {
            panic!("Expected Obj");
        }
    }
}
