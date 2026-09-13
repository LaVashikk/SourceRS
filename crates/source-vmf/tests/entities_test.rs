#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use pretty_assertions::assert_eq;
    use vmf_forge::VmfBlock;
    use vmf_forge::VmfSerializable;
    use vmf_forge::errors::VmfError;
    use vmf_forge::vmf::common::Editor;
    use vmf_forge::vmf::entities::*;

    // Tests for Entity
    #[test]
    fn entity_try_from_valid_block() {
        let mut key_values = IndexMap::new();
        key_values.insert("classname".to_string(), "logic_relay".to_string());
        key_values.insert("targetname".to_string(), "test_relay".to_string());

        let mut block = VmfBlock {
            name: "entity".to_string(),
            key_values,
            blocks: Vec::new(),
        };

        let editor = VmfBlock {
            name: "editor".to_string(),
            key_values: {
                let mut map = IndexMap::new();
                map.insert("color".to_string(), "255 255 255".to_string());
                map
            },
            blocks: Vec::new(),
        };

        let solid1 = VmfBlock {
            name: "solid".to_string(),
            key_values: {
                let mut map = IndexMap::new();
                map.insert("id".to_string(), "1".to_string());
                map
            },
            blocks: vec![],
        };

        let hidden = VmfBlock {
            name: "hidden".to_string(),
            key_values: IndexMap::new(),
            blocks: vec![VmfBlock {
                name: "solid".to_string(),
                key_values: {
                    let mut map = IndexMap::new();
                    map.insert("id".to_string(), "2".to_string());
                    map
                },
                blocks: Vec::new(),
            }],
        };

        block.blocks.push(editor);
        block.blocks.push(solid1);
        block.blocks.push(hidden);

        let entity = Entity::try_from(block).unwrap();

        assert_eq!(entity.key_values.get("classname").unwrap(), "logic_relay");
        assert_eq!(entity.key_values.get("targetname").unwrap(), "test_relay");
        assert_eq!(entity.editor.color, "255 255 255");
        assert_eq!(entity.solids.as_ref().unwrap().len(), 2);
        assert_eq!(entity.solids.as_ref().unwrap()[0].id, 1);
        assert_eq!(entity.solids.as_ref().unwrap()[1].id, 2);
    }

    #[test]
    fn entity_try_from_invalid_type() {
        let mut key_values = IndexMap::new();
        key_values.insert("classname".to_string(), "logic_relay".to_string());
        key_values.insert("targetname".to_string(), "abc".to_string());

        let block = VmfBlock {
            name: "entity".to_string(),
            key_values,
            blocks: vec![VmfBlock {
                name: "solid".to_string(),
                key_values: {
                    let mut map = IndexMap::new();
                    map.insert("id".to_string(), "abc".to_string());
                    map
                },
                blocks: vec![],
            }],
        };

        let result = Entity::try_from(block);
        assert!(matches!(
            result,
            Err(VmfError::ParseInt { source: _, key: _ })
        ));
    }

    #[test] // todo: fuck u, IndexMap (unsorted)!
    fn entity_to_vmf_string() {
        let entity = Entity {
            key_values: {
                let mut map = IndexMap::new();
                map.insert("classname".to_string(), "logic_relay".to_string());
                map.insert("targetname".to_string(), "test_relay".to_string());
                map
            },
            connections: None,
            solids: None,
            editor: Editor {
                color: "255 255 255".to_string(),
                ..Default::default()
            },
            is_hidden: false,
        };

        let expected = "\
        entity\n\
        {\n\
        \t\"classname\" \"logic_relay\"\n\
        \t\"targetname\" \"test_relay\"\n\
        \teditor\n\
        \t{\n\
        \t\t\"color\" \"255 255 255\"\n\
        \t\t\"visgroupshown\" \"1\"\n\
        \t\t\"visgroupautoshown\" \"1\"\n\
        \t}\n\
        }\n";

        assert_eq!(entity.to_vmf_string(0), expected);
    }

    #[test]
    fn entity_into_vmf_block() {
        let entity = Entity {
            key_values: {
                let mut map = IndexMap::new();
                map.insert("classname".to_string(), "logic_relay".to_string());
                map.insert("targetname".to_string(), "test_relay".to_string());
                map
            },
            connections: None,
            solids: None,
            editor: Editor {
                color: "255 255 255".to_string(),
                ..Default::default()
            },
            is_hidden: false,
        };

        let block: VmfBlock = entity.into();

        assert_eq!(block.name, "entity");
        assert_eq!(
            block.key_values.get("classname"),
            Some(&"logic_relay".to_string())
        );
        assert_eq!(
            block.key_values.get("targetname"),
            Some(&"test_relay".to_string())
        );
        assert_eq!(block.blocks.len(), 1);
        assert_eq!(block.blocks[0].name, "editor");
        assert_eq!(
            block.blocks[0].key_values.get("color"),
            Some(&"255 255 255".to_string())
        );
    }

    // Tests for Entities
    #[test]
    fn test_entities_find_by_keyvalue() {
        let mut entities = Entities::default();
        entities.push(Entity {
            key_values: {
                let mut map = IndexMap::new();
                map.insert("classname".to_string(), "entity1".to_string());
                map.insert("key1".to_string(), "value1".to_string());
                map
            },
            ..Default::default()
        });
        entities.push(Entity {
            key_values: {
                let mut map = IndexMap::new();
                map.insert("classname".to_string(), "entity2".to_string());
                map.insert("key1".to_string(), "value2".to_string());
                map
            },
            ..Default::default()
        });
        entities.push(Entity {
            key_values: {
                let mut map = IndexMap::new();
                map.insert("classname".to_string(), "entity1".to_string());
                map.insert("key2".to_string(), "value3".to_string());
                map
            },
            ..Default::default()
        });

        let found_entities: Vec<&Entity> =
            entities.find_by_keyvalue("classname", "entity1").collect();
        assert_eq!(found_entities.len(), 2);

        let found_entities: Vec<&Entity> = entities.find_by_keyvalue("key1", "value1").collect();
        assert_eq!(found_entities.len(), 1);

        let found_entities: Vec<&Entity> = entities.find_by_keyvalue("key3", "value4").collect();
        assert_eq!(found_entities.len(), 0);
    }

    #[test]
    fn test_entities_find_by_keyvalue_mut() {
        let mut entities = Entities::default();
        let entity1 = Entity {
            key_values: {
                let mut map = IndexMap::new();
                map.insert("classname".to_string(), "entity1".to_string());
                map.insert("key1".to_string(), "value1".to_string());
                map
            },
            ..Default::default()
        };
        let entity2 = Entity {
            key_values: {
                let mut map = IndexMap::new();
                map.insert("classname".to_string(), "entity2".to_string());
                map.insert("key1".to_string(), "value2".to_string());
                map
            },
            ..Default::default()
        };
        entities.push(entity1);
        entities.push(entity2);

        let found_entities: Vec<_> = entities
            .find_by_keyvalue_mut("classname", "entity1")
            .collect();
        assert_eq!(found_entities[0].key_values.get("key1").unwrap(), "value1");
        assert_eq!(found_entities.len(), 1);

        assert_eq!(entities[1].key_values.get("key1").unwrap(), "value2");
    }

    #[test]
    fn test_entities_find_by_classname() {
        let mut entities = Entities::default();
        entities.push(Entity {
            key_values: {
                let mut map = IndexMap::new();
                map.insert("classname".to_string(), "info_player_start".to_string());
                map
            },
            ..Default::default()
        });
        entities.push(Entity {
            key_values: {
                let mut map = IndexMap::new();
                map.insert("classname".to_string(), "entity2".to_string());
                map
            },
            ..Default::default()
        });

        let found_entities: Vec<&Entity> =
            entities.find_by_classname("info_player_start").collect();
        assert_eq!(found_entities.len(), 1);
    }

    #[test]
    fn test_entities_find_by_name() {
        let mut entities = Entities::default();
        entities.push(Entity {
            key_values: {
                let mut map = IndexMap::new();
                map.insert("targetname".to_string(), "my_entity".to_string());
                map
            },
            ..Default::default()
        });
        entities.push(Entity {
            key_values: {
                let mut map = IndexMap::new();
                map.insert("targetname".to_string(), "another_entity".to_string());
                map
            },
            ..Default::default()
        });

        let found_entities: Vec<&Entity> = entities.find_by_name("my_entity").collect();
        assert_eq!(found_entities.len(), 1);
    }

    #[test]
    fn test_entities_find_by_classname_mut() {
        let mut entities = Entities::default();
        entities.push(Entity {
            key_values: {
                let mut map = IndexMap::new();
                map.insert("classname".to_string(), "info_player_start".to_string());
                map
            },
            ..Default::default()
        });

        let mut found_entities = entities.find_by_classname_mut("info_player_start");
        assert!(found_entities.next().is_some());
        assert!(found_entities.next().is_none());
    }

    #[test]
    fn test_entities_find_by_name_mut() {
        let mut entities = Entities::default();
        entities.push(Entity {
            key_values: {
                let mut map = IndexMap::new();
                map.insert("targetname".to_string(), "my_entity".to_string());
                map
            },
            ..Default::default()
        });

        let mut found_entities = entities.find_by_name_mut("my_entity");
        assert!(found_entities.next().is_some());
        assert!(found_entities.next().is_none());
    }

    #[test]
    fn entity_to_vmf_string_with_connections() {
        let entity = Entity {
            key_values: {
                let mut map = IndexMap::new();
                map.insert("id".to_string(), "2810".to_string());
                map.insert("classname".to_string(), "logic_relay".to_string());
                map.insert(
                    "targetname".to_string(),
                    "button_unpressed_relay".to_string(),
                );
                map.insert("origin".to_string(), "304 416 64".to_string());
                map
            },
            connections: Some({
                vec![
                    (
                        "OnTrigger".to_string(),
                        "@exit_door instance:door_close_relay;Trigger  0 -1".to_string(),
                    ),
                    (
                        "OnTrigger".to_string(),
                        "door_checkmark Uncheck  0 -1".to_string(),
                    ),
                ]
            }),
            solids: None,
            editor: Editor {
                color: "220 30 220".to_string(),
                visgroup_shown: true,
                visgroup_auto_shown: true,
                logical_pos: Some("[0 -5268]".to_string()),
                ..Default::default()
            },
            is_hidden: false,
        };

        let expected = "\
        entity\n\
        {\n\
        \t\"id\" \"2810\"\n\
        \t\"classname\" \"logic_relay\"\n\
        \t\"targetname\" \"button_unpressed_relay\"\n\
        \t\"origin\" \"304 416 64\"\n\
        \tconnections\n\
        \t{\n\
        \t\t\"OnTrigger\" \"@exit_door instance:door_close_relay;Trigger  0 -1\"\n\
        \t\t\"OnTrigger\" \"door_checkmark Uncheck  0 -1\"\n\
        \t}\n\
        \teditor\n\
        \t{\n\
        \t\t\"color\" \"220 30 220\"\n\
        \t\t\"visgroupshown\" \"1\"\n\
        \t\t\"visgroupautoshown\" \"1\"\n\
        \t\t\"logicalpos\" \"[0 -5268]\"\n\
        \t}\n\
        }\n";

        assert_eq!(entity.to_vmf_string(0), expected);
    }

    // NEW tests (ver 3)
    #[test]
    fn entity_new() {
        let entity = Entity::new("info_player_start", 1);
        assert_eq!(entity.classname(), Some("info_player_start"));
        assert_eq!(entity.id(), 1);
        assert!(entity.connections.is_none());
        assert!(entity.solids.is_none());
        assert_eq!(entity.editor.color, "255 255 255"); // Check default editor
        assert!(!entity.is_hidden);
    }

    #[test]
    fn entity_set() {
        let mut entity = Entity::new("info_player_start", 1);
        entity.set("targetname".to_string(), "my_player_start".to_string());
        assert_eq!(entity.targetname(), Some("my_player_start"));

        entity.set("origin".to_string(), "10 20 30".to_string());
        assert_eq!(entity.get("origin"), Some(&"10 20 30".to_string()));
    }

    #[test]
    fn entity_remove_key() {
        let mut entity = Entity::new("info_player_start", 1);
        entity.set("targetname".to_string(), "my_player_start".to_string());

        let removed_value = entity.remove_key("targetname");
        assert_eq!(removed_value, Some("my_player_start".to_string()));
        assert!(entity.targetname().is_none());

        let none_value = entity.remove_key("nonexistent_key");
        assert!(none_value.is_none());
    }

    #[test]
    fn entity_swap_remove_key() {
        let mut entity = Entity::new("info_player_start", 1);
        entity.set("targetname".to_string(), "my_player_start".to_string());

        let removed_value = entity.swap_remove_key("targetname");
        assert_eq!(removed_value, Some("my_player_start".to_string()));
        assert!(entity.targetname().is_none());

        let none_value = entity.swap_remove_key("nonexistent_key");
        assert!(none_value.is_none());
    }

    #[test]
    fn entity_get() {
        let mut entity = Entity::new("info_player_start", 1);
        entity.set("targetname".to_string(), "my_player_start".to_string());

        assert_eq!(
            entity.get("targetname"),
            Some(&"my_player_start".to_string())
        );
        assert!(entity.get("nonexistent_key").is_none());
    }

    #[test]
    fn entity_get_mut() {
        let mut entity = Entity::new("info_player_start", 1);
        entity.set("targetname".to_string(), "my_player_start".to_string());

        if let Some(targetname) = entity.get_mut("targetname") {
            *targetname = "new_targetname".to_string();
        }

        assert_eq!(entity.targetname(), Some("new_targetname"));
        assert!(entity.get_mut("nonexistent_key").is_none());
    }

    #[test]
    fn entity_classname_targetname_id_model() {
        let mut entity = Entity::new("info_player_start", 1);
        entity.set("targetname".to_string(), "my_player_start".to_string());
        entity.set("model".to_string(), "*1".to_string());

        assert_eq!(entity.classname(), Some("info_player_start"));
        assert_eq!(entity.targetname(), Some("my_player_start"));
        assert_eq!(entity.id(), 1);
        assert_eq!(entity.model(), Some("*1"));

        let mut entity2 = Entity::new("func_brush", 2); // different classname
        entity2.set("targetname".to_string(), "my_brush".to_string());
        assert_eq!(entity2.classname(), Some("func_brush"));
    }

    #[test]
    fn entity_add_connection() {
        let mut entity = Entity::new("logic_relay", 1);
        entity.add_connection("OnTrigger", "my_door", "Open", "", 0.0, -1);
        entity.add_connection("OnTrigger", "my_sound", "PlaySound", "bang", 0.5, 1);

        assert!(entity.connections.is_some());
        let connections = entity.connections.unwrap();
        assert_eq!(connections.len(), 2);
        assert_eq!(
            connections[0],
            (
                "OnTrigger".to_string(),
                "my_door\x1BOpen\x1B\x1B0\x1B-1".to_string()
            )
        );
        assert_eq!(
            connections[1],
            (
                "OnTrigger".to_string(),
                "my_sound\x1BPlaySound\x1Bbang\x1B0.5\x1B1".to_string()
            )
        );
    }

    #[test]
    fn entity_has_connection() {
        let mut entity = Entity::new("logic_relay", 1);
        entity.add_connection("OnTrigger", "my_door", "Open", "", 0.0, -1);

        assert!(entity.has_connection("OnTrigger", "my_door\x1BOpen\x1B\x1B0\x1B-1"));
        assert!(!entity.has_connection("OnTrigger", "my_door\x1BClose\x1B\x1B0\x1B-1"));
        assert!(!entity.has_connection("OnStartTouch", "my_door\x1BOpen\x1B\x1B0\x1B-1"));
    }

    #[test]
    fn entity_clear_connections() {
        let mut entity = Entity::new("logic_relay", 1);
        entity.add_connection("OnTrigger", "my_door", "Open", "", 0.0, -1);
        entity.clear_connections();
        assert!(entity.connections.is_none());
    }

    // Tests for Entities (the collection)

    #[test]
    fn entities_remove_entity() {
        let mut entities = Entities::default();
        let entity1 = Entity::new("info_player_start", 1);
        let entity2 = Entity::new("func_detail", 2);
        entities.push(entity1.clone());
        entities.push(entity2);

        let removed_entity = entities.remove_entity(1);
        assert_eq!(removed_entity, Some(entity1));
        assert_eq!(entities.len(), 1);

        let non_existent = entities.remove_entity(3);
        assert!(non_existent.is_none());
    }

    #[test]
    fn test_remove_by_keyvalue() {
        let mut entities = Entities::default();
        entities.push(Entity::new("info_player_start", 1));
        entities.push(Entity::new("info_player_deathmatch", 2));
        entities.push(Entity::new("info_player_start", 3));

        entities.remove_by_keyvalue("classname", "info_player_start");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].id(), 2);
    }

    #[test]
    fn remove_by_keyvalue_no_match() {
        let mut entities = Entities::default();
        entities.push(Entity::new("info_player_start", 1));
        entities.remove_by_keyvalue("classname", "func_door"); // No entities with this classname
        assert_eq!(entities.len(), 1);
    }
    #[test]
    fn remove_by_keyvalue_empty() {
        let mut entities = Entities::default();
        entities.remove_by_keyvalue("classname", "anything"); // Should not panic
        assert_eq!(entities.len(), 0);
    }
}
