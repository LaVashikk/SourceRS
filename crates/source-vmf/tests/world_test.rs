#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use pretty_assertions::assert_eq;
    use source_vmf::VmfBlock;
    use source_vmf::VmfSerializable;
    use source_vmf::vmf::common::Editor;
    use source_vmf::vmf::world::*;
    use source_vmf::{FromBlock, ParseCtx};

    // Tests for World
    #[test]
    fn world_try_from_valid_block() {
        let mut key_values = IndexMap::new();
        key_values.insert("classname".to_string(), "worldspawn".to_string());

        let mut block = VmfBlock {
            name: "world".to_string(),
            key_values,
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

        let solid2 = VmfBlock {
            name: "solid".to_string(),
            key_values: {
                let mut map = IndexMap::new();
                map.insert("id".to_string(), "2".to_string());
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
                    map.insert("id".to_string(), "3".to_string());
                    map
                },
                blocks: vec![],
            }],
        };
        let group = VmfBlock {
            name: "group".to_string(),
            key_values: {
                let mut map = IndexMap::new();
                map.insert("id".to_string(), "10".to_string());
                map
            },
            blocks: vec![],
        };

        block.blocks.push(solid1);
        block.blocks.push(solid2);
        block.blocks.push(hidden);
        block.blocks.push(group);

        let world = World::try_from(block).unwrap();

        assert_eq!(world.key_values.get("classname").unwrap(), "worldspawn");
        assert_eq!(world.solids.len(), 2);
        assert_eq!(world.solids[0].id, 1);
        assert_eq!(world.solids[1].id, 2);
        assert_eq!(world.hidden.len(), 1);
        assert_eq!(world.hidden[0].id, 3);
        assert_eq!(world.groups[0].id, 10);
    }

    #[test]
    fn world_try_from_invalid_type() {
        let mut key_values = IndexMap::new();
        key_values.insert("classname".to_string(), "worldspawn".to_string());
        let block = VmfBlock {
            name: "world".to_string(),
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

        let mut ctx = ParseCtx::default();
        let world = World::from_block(block, &mut ctx);

        assert_eq!(world.solids.len(), 1);
        assert_eq!(world.solids[0].id, 0);
        assert!(ctx.warnings[0].starts_with("world/solid[0]:"));
    }

    #[test]
    fn world_to_vmf_string() {
        let world = World {
            key_values: {
                let mut map = IndexMap::new();
                map.insert("classname".to_string(), "worldspawn".to_string());
                map
            },
            solids: vec![
                Solid {
                    id: 1,
                    sides: vec![],
                    editor: Editor::default(),
                    extra: Default::default(),
                },
                Solid {
                    id: 2,
                    sides: vec![],
                    editor: Editor::default(),
                    extra: Default::default(),
                },
            ],
            hidden: vec![Solid {
                id: 3,
                sides: vec![],
                editor: Editor::default(),
                extra: Default::default(),
            }],
            groups: vec![Group {
                id: 10,
                editor: Editor::default(),
                extra: Default::default(),
            }],
            extra: Default::default(),
        };

        let expected = "\
        world\n\
        {\n\
        \t\"classname\" \"worldspawn\"\n\
        \tsolid\n\
        \t{\n\
        \t\t\"id\" \"1\"\n\
        \t\teditor\n\
        \t\t{\n\
        \t\t\t\"color\" \"255 255 255\"\n\
        \t\t\t\"visgroupshown\" \"1\"\n\
        \t\t\t\"visgroupautoshown\" \"1\"\n\
        \t\t}\n\
        \t}\n\
        \tsolid\n\
        \t{\n\
        \t\t\"id\" \"2\"\n\
        \t\teditor\n\
        \t\t{\n\
          \t\t\t\"color\" \"255 255 255\"\n\
         \t\t\t\"visgroupshown\" \"1\"\n\
        \t\t\t\"visgroupautoshown\" \"1\"\n\
        \t\t}\n\
        \t}\n\
        \thidden\n\
        \t{\n\
        \t\tsolid\n\
        \t\t{\n\
        \t\t\t\"id\" \"3\"\n\
         \t\t\teditor\n\
        \t\t\t{\n\
          \t\t\t\t\"color\" \"255 255 255\"\n\
        \t\t\t\t\"visgroupshown\" \"1\"\n\
        \t\t\t\t\"visgroupautoshown\" \"1\"\n\
        \t\t\t}\n\
        \t\t}\n\
        \t}\n\
        \tgroup\n\
        \t{\n\
        \t\t\"id\" \"10\"\n\
         \t\teditor\n\
        \t\t{\n\
         \t\t\t\"color\" \"255 255 255\"\n\
        \t\t\t\"visgroupshown\" \"1\"\n\
        \t\t\t\"visgroupautoshown\" \"1\"\n\
        \t\t}\n\
        \t}\n\
        }\n";

        assert_eq!(world.to_vmf_string(0), expected);
    }

    #[test]
    fn world_into_vmf_block() {
        let world = World {
            key_values: {
                let mut map = IndexMap::new();
                map.insert("classname".to_string(), "worldspawn".to_string());
                map
            },
            solids: vec![
                Solid {
                    id: 1,
                    sides: vec![],
                    editor: Editor::default(),
                    extra: Default::default(),
                },
                Solid {
                    id: 2,
                    sides: vec![],
                    editor: Editor::default(),
                    extra: Default::default(),
                },
            ],
            hidden: vec![Solid {
                id: 3,
                sides: vec![],
                editor: Editor::default(),
                extra: Default::default(),
            }],
            groups: vec![Group {
                id: 10,
                editor: Editor::default(),
                extra: Default::default(),
            }],
            extra: Default::default(),
        };
        let block: VmfBlock = world.into();

        assert_eq!(block.name, "world");
        assert_eq!(
            block.key_values.get("classname"),
            Some(&"worldspawn".to_string())
        );
        assert_eq!(block.blocks.len(), 4);
        assert_eq!(block.blocks[0].name, "solid");
        assert_eq!(block.blocks[1].name, "solid");
        assert_eq!(block.blocks[2].name, "hidden");
        assert_eq!(block.blocks[3].name, "group");
    }

    // Tests for Solid
    #[test]
    fn solid_try_from_valid_block() {
        let mut key_values = IndexMap::new();
        key_values.insert("id".to_string(), "1".to_string());

        let block = VmfBlock {
            name: "solid".to_string(),
            key_values,
            blocks: vec![],
        };

        let solid = Solid::try_from(block).unwrap();
        assert_eq!(solid.id, 1);
    }

    #[test]
    fn solid_missing_key_defaults_and_warns() {
        let block = VmfBlock {
            name: "solid".to_string(),
            key_values: IndexMap::new(),
            blocks: vec![],
        };

        let mut ctx = ParseCtx::default();
        let solid = Solid::from_block(block, &mut ctx);

        assert_eq!(solid.id, 0);
        assert!(ctx.warnings[0].contains("missing 'id'"));
    }

    #[test]
    fn solid_invalid_type_defaults_and_warns() {
        let mut key_values = IndexMap::new();
        key_values.insert("id".to_string(), "abc".to_string());

        let block = VmfBlock {
            name: "solid".to_string(),
            key_values,
            blocks: vec![],
        };

        let mut ctx = ParseCtx::default();
        let solid = Solid::from_block(block, &mut ctx);

        assert_eq!(solid.id, 0);
        assert!(ctx.warnings[0].contains("unparsable value 'abc'"));
    }

    #[test]
    fn solid_keeps_unknown_keys_and_blocks() {
        let mut key_values = IndexMap::new();
        key_values.insert("id".to_string(), "1".to_string());
        key_values.insert("vendor_flag".to_string(), "7".to_string());

        let block = VmfBlock {
            name: "solid".to_string(),
            key_values,
            blocks: vec![VmfBlock {
                name: "vendor_block".to_string(),
                key_values: IndexMap::new(),
                blocks: vec![],
            }],
        };

        let mut ctx = ParseCtx::default();
        let solid = Solid::from_block(block, &mut ctx);

        assert_eq!(solid.id, 1);
        assert_eq!(
            solid.extra.key_values.get("vendor_flag"),
            Some(&"7".to_string())
        );
        assert_eq!(solid.extra.blocks.len(), 1);

        let text = solid.to_vmf_string(0);
        assert!(text.contains("vendor_flag"));
        assert!(text.contains("vendor_block"));
    }

    #[test]
    fn solid_to_vmf_string() {
        let solid = Solid {
            id: 1,
            sides: vec![],
            editor: Editor::default(),
            extra: Default::default(),
        };

        let expected = "\
        solid\n\
        {\n\
        \t\"id\" \"1\"\n\
        \teditor\n\
        \t{\n\
        \t\t\"color\" \"255 255 255\"\n\
        \t\t\"visgroupshown\" \"1\"\n\
        \t\t\"visgroupautoshown\" \"1\"\n\
        \t}\n\
        }\n";
        assert_eq!(solid.to_vmf_string(0), expected);
    }

    #[test]
    fn solid_into_vmf_block() {
        let solid = Solid {
            id: 1,
            sides: vec![],
            editor: Editor::default(),
            extra: Default::default(),
        };
        let block: VmfBlock = solid.into();

        assert_eq!(block.name, "solid");
        assert_eq!(block.key_values.get("id"), Some(&"1".to_string()));
        assert!(block.blocks.len() == 1);
        assert_eq!(block.blocks[0].name, "editor");
    }

    // Tests for Side
    #[test]
    fn side_try_from_valid_block() {
        let mut key_values = IndexMap::new();
        key_values.insert("id".to_string(), "1".to_string());
        key_values.insert("plane".to_string(), "(0 0 0) (1 0 0) (0 1 0)".to_string());
        key_values.insert("material".to_string(), "test_material".to_string());
        key_values.insert("uaxis".to_string(), "[1 0 0 0.5] 0.25".to_string());
        key_values.insert("vaxis".to_string(), "[0 1 0 0.5] 0.25".to_string());
        key_values.insert("lightmapscale".to_string(), "16".to_string());
        key_values.insert("smoothing_groups".to_string(), "1".to_string());

        let block = VmfBlock {
            name: "side".to_string(),
            key_values,
            blocks: Vec::new(),
        };

        let side = Side::try_from(block).unwrap();

        assert_eq!(side.id, 1);
        assert_eq!(side.plane, "(0 0 0) (1 0 0) (0 1 0)");
        assert_eq!(side.material, "test_material");
        assert_eq!(side.u_axis, "[1 0 0 0.5] 0.25");
        assert_eq!(side.v_axis, "[0 1 0 0.5] 0.25");
        assert_eq!(side.lightmap_scale, 16);
        assert_eq!(side.smoothing_groups, 1);
    }

    #[test]
    fn side_try_from_missing_key() {
        let mut key_values = IndexMap::new();
        key_values.insert("plane".to_string(), "(0 0 0) (1 0 0) (0 1 0)".to_string());
        key_values.insert("material".to_string(), "test_material".to_string());
        key_values.insert("uaxis".to_string(), "[1 0 0 0.5] 0.25".to_string());
        key_values.insert("vaxis".to_string(), "[0 1 0 0.5] 0.25".to_string());
        key_values.insert("lightmapscale".to_string(), "16".to_string());
        key_values.insert("smoothing_groups".to_string(), "1".to_string());

        let block = VmfBlock {
            name: "side".to_string(),
            key_values,
            blocks: Vec::new(),
        };

        let mut ctx = ParseCtx::default();
        let side = Side::from_block(block, &mut ctx);

        assert_eq!(side.id, 0);
        assert_eq!(side.material, "test_material");
        assert_eq!(side.smoothing_groups, 1);
        assert!(ctx.warnings[0].contains("missing 'id'"));
    }

    #[test]
    fn side_invalid_type_defaults_and_warns() {
        let mut key_values = IndexMap::new();
        key_values.insert("id".to_string(), "abc".to_string());
        key_values.insert("plane".to_string(), "(0 0 0) (1 0 0) (0 1 0)".to_string());
        key_values.insert("material".to_string(), "test_material".to_string());
        key_values.insert("uaxis".to_string(), "[1 0 0 0.5] 0.25".to_string());
        key_values.insert("vaxis".to_string(), "[0 1 0 0.5] 0.25".to_string());
        key_values.insert("lightmapscale".to_string(), "16".to_string());
        key_values.insert("smoothing_groups".to_string(), "1".to_string());

        let block = VmfBlock {
            name: "side".to_string(),
            key_values,
            blocks: Vec::new(),
        };

        let mut ctx = ParseCtx::default();
        let side = Side::from_block(block, &mut ctx);

        assert_eq!(side.id, 0);
        assert_eq!(side.plane, "(0 0 0) (1 0 0) (0 1 0)");
        assert!(ctx.warnings[0].contains("unparsable value 'abc'"));
    }

    #[test]
    fn side_to_vmf_string() {
        let side = Side {
            id: 1,
            plane: "(0 0 0) (1 0 0) (0 1 0)".to_string(),
            material: "test_material".to_string(),
            u_axis: "[1 0 0 0.5] 0.25".to_string(),
            v_axis: "[0 1 0 0.5] 0.25".to_string(),
            rotation: None,
            lightmap_scale: 16,
            smoothing_groups: 1,
            flags: None,
            dispinfo: None,
            extra: Default::default(),
        };
        let expected = "\
        side\n\
        {\n\
        \t\"id\" \"1\"\n\
        \t\"plane\" \"(0 0 0) (1 0 0) (0 1 0)\"\n\
        \t\"material\" \"test_material\"\n\
        \t\"uaxis\" \"[1 0 0 0.5] 0.25\"\n\
        \t\"vaxis\" \"[0 1 0 0.5] 0.25\"\n\
        \t\"lightmapscale\" \"16\"\n\
        \t\"smoothing_groups\" \"1\"\n\
        }\n";

        assert_eq!(side.to_vmf_string(0), expected);
    }

    #[test]
    fn side_into_vmf_block() {
        let side = Side {
            id: 1,
            plane: "(0 0 0) (1 0 0) (0 1 0)".to_string(),
            material: "test_material".to_string(),
            u_axis: "[1 0 0 0.5] 0.25".to_string(),
            v_axis: "[0 1 0 0.5] 0.25".to_string(),
            rotation: None,
            lightmap_scale: 16,
            smoothing_groups: 1,
            flags: None,
            dispinfo: None,
            extra: Default::default(),
        };
        let block: VmfBlock = side.into();

        assert_eq!(block.name, "side");
        assert_eq!(block.key_values.get("id"), Some(&"1".to_string()));
        assert_eq!(
            block.key_values.get("plane"),
            Some(&"(0 0 0) (1 0 0) (0 1 0)".to_string())
        );
        assert_eq!(
            block.key_values.get("material"),
            Some(&"test_material".to_string())
        );
        assert_eq!(
            block.key_values.get("uaxis"),
            Some(&"[1 0 0 0.5] 0.25".to_string())
        );
        assert_eq!(
            block.key_values.get("vaxis"),
            Some(&"[0 1 0 0.5] 0.25".to_string())
        );
        assert_eq!(
            block.key_values.get("lightmapscale"),
            Some(&"16".to_string())
        );
        assert_eq!(
            block.key_values.get("smoothing_groups"),
            Some(&"1".to_string())
        );
    }

    // Tests for Group
    #[test]
    fn group_try_from_valid_block() {
        let mut key_values = IndexMap::new();
        key_values.insert("id".to_string(), "1".to_string());

        let block = VmfBlock {
            name: "group".to_string(),
            key_values,
            blocks: vec![],
        };
        let group = Group::try_from(block).unwrap();
        assert_eq!(group.id, 1);
    }

    #[test]
    fn group_missing_key_defaults_and_warns() {
        let block = VmfBlock {
            name: "group".to_string(),
            key_values: IndexMap::new(),
            blocks: vec![],
        };

        let mut ctx = ParseCtx::default();
        let group = Group::from_block(block, &mut ctx);

        assert_eq!(group.id, 0);
        assert!(ctx.warnings[0].contains("missing 'id'"));
    }

    #[test]
    fn group_invalid_type_defaults_and_warns() {
        let mut key_values = IndexMap::new();
        key_values.insert("id".to_string(), "abc".to_string());

        let block = VmfBlock {
            name: "group".to_string(),
            key_values,
            blocks: vec![],
        };

        let mut ctx = ParseCtx::default();
        let group = Group::from_block(block, &mut ctx);

        assert_eq!(group.id, 0);
        assert!(ctx.warnings[0].contains("unparsable value 'abc'"));
    }
    #[test]
    fn group_to_vmf_string() {
        let group = Group {
            id: 1,
            editor: Editor::default(),
            extra: Default::default(),
        };
        let expected = "\
        group\n\
        {\n\
        \t\"id\" \"1\"\n\
        \teditor\n\
        \t{\n\
        \t\t\"color\" \"255 255 255\"\n\
        \t\t\"visgroupshown\" \"1\"\n\
        \t\t\"visgroupautoshown\" \"1\"\n\
        \t}\n\
        }\n";
        assert_eq!(group.to_vmf_string(0), expected);
    }

    #[test]
    fn group_into_vmf_block() {
        let group = Group {
            id: 1,
            editor: Editor::default(),
            extra: Default::default(),
        };
        let block: VmfBlock = group.into();
        assert_eq!(block.name, "group");
        assert_eq!(block.key_values.get("id"), Some(&"1".to_string()));
        assert!(block.blocks.len() == 1);
        assert_eq!(block.blocks[0].name, "editor");
    }
}

#[cfg(test)]
mod hidden_solids {
    use pretty_assertions::assert_eq;
    use source_vmf::prelude::*;

    const MAP: &str = r#"
versioninfo
{
	"editorversion" "400"
}
world
{
	"id" "1"
	"classname" "worldspawn"
	hidden
	{
		solid
		{
			"id" "10"
			side
			{
				"id" "11"
				"plane" "(0 0 0) (1 0 0) (0 1 0)"
				"material" "DEV/DEV_A"
			}
		}
	}
	hidden
	{
		solid
		{
			"id" "20"
			side
			{
				"id" "21"
				"plane" "(0 0 8) (1 0 8) (0 1 8)"
				"material" "DEV/DEV_B"
			}
		}
	}
}
"#;

    #[test]
    fn every_hidden_solid_survives_a_save_and_reload() {
        let once = VmfFile::parse(MAP).unwrap();
        assert_eq!(once.world.hidden.len(), 2, "both read from the file");

        let twice = VmfFile::parse(&once.to_vmf_string()).unwrap();
        assert_eq!(twice.world.hidden.len(), 2, "and both survive a round trip");
        assert_eq!(once.to_vmf_string(), twice.to_vmf_string());
    }

    #[test]
    fn several_solids_in_one_wrapper_are_all_read() {
        let grouped = MAP.replace("\t}\n\thidden\n\t{\n", "\n");
        let file = VmfFile::parse(&grouped).unwrap();
        assert_eq!(file.world.hidden.len(), 2);
    }
}
