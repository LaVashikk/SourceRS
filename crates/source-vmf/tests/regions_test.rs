#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use pretty_assertions::assert_eq;
    use vmf_forge::VmfBlock;
    use vmf_forge::VmfSerializable;
    use vmf_forge::errors::VmfError;
    use vmf_forge::vmf::regions::*;

    // Tests for Cameras
    #[test]
    fn cameras_try_from_valid_block() {
        let mut key_values = IndexMap::new();
        key_values.insert("activecamera".to_string(), "1".to_string());

        let mut block = VmfBlock {
            name: "cameras".to_string(),
            key_values,
            blocks: Vec::new(),
        };

        let cam1 = VmfBlock {
            name: "camera".to_string(),
            key_values: {
                let mut map = IndexMap::new();
                map.insert("position".to_string(), "0 0 0".to_string());
                map.insert("look".to_string(), "1 0 0".to_string());
                map
            },
            blocks: Vec::new(),
        };
        let cam2 = VmfBlock {
            name: "camera".to_string(),
            key_values: {
                let mut map = IndexMap::new();
                map.insert("position".to_string(), "0 1 0".to_string());
                map.insert("look".to_string(), "0 1 0".to_string());
                map
            },
            blocks: Vec::new(),
        };

        block.blocks.push(cam1);
        block.blocks.push(cam2);

        let cameras = Cameras::try_from(block).unwrap();

        assert_eq!(cameras.active, 1);
        assert_eq!(cameras.cams.len(), 2);
        assert_eq!(cameras.cams[0].position, "0 0 0");
        assert_eq!(cameras.cams[0].look, "1 0 0");
        assert_eq!(cameras.cams[1].position, "0 1 0");
        assert_eq!(cameras.cams[1].look, "0 1 0");
    }

    #[test]
    fn cameras_try_from_missing_key() {
        let block = VmfBlock {
            name: "cameras".to_string(),
            key_values: IndexMap::new(),
            blocks: Vec::new(),
        };

        let result = Cameras::try_from(block);

        assert!(matches!(result, Err(VmfError::InvalidFormat(_))));
    }

    #[test]
    fn cameras_try_from_invalid_type() {
        let mut key_values = IndexMap::new();
        key_values.insert("activecamera".to_string(), "abc".to_string());

        let block = VmfBlock {
            name: "cameras".to_string(),
            key_values,
            blocks: Vec::new(),
        };

        let result = Cameras::try_from(block);
        assert!(matches!(
            result,
            Err(VmfError::ParseInt { source: _, key: _ })
        ));
    }

    #[test]
    fn cameras_into_vmf_block() {
        let cameras = Cameras {
            active: 1,
            cams: vec![
                Camera {
                    position: "0 0 0".to_string(),
                    look: "1 0 0".to_string(),
                },
                Camera {
                    position: "0 1 0".to_string(),
                    look: "0 1 0".to_string(),
                },
            ],
        };
        let block: VmfBlock = cameras.into();

        assert_eq!(block.name, "cameras");
        assert_eq!(block.key_values.get("active"), Some(&"1".to_string()));
        assert_eq!(block.blocks.len(), 2);
    }

    // Tests for Camera
    #[test]
    fn camera_try_from_valid_block() {
        let mut key_values = IndexMap::new();
        key_values.insert("position".to_string(), "0 0 0".to_string());
        key_values.insert("look".to_string(), "1 0 0".to_string());

        let block = VmfBlock {
            name: "camera".to_string(),
            key_values,
            blocks: Vec::new(),
        };

        let camera = Camera::try_from(block).unwrap();

        assert_eq!(camera.position, "0 0 0");
        assert_eq!(camera.look, "1 0 0");
    }

    #[test]
    fn camera_try_from_missing_key() {
        let mut key_values = IndexMap::new();
        key_values.insert("look".to_string(), "1 0 0".to_string());

        let block = VmfBlock {
            name: "camera".to_string(),
            key_values,
            blocks: Vec::new(),
        };

        let result = Camera::try_from(block);

        assert!(matches!(result, Err(VmfError::InvalidFormat(_))));
    }

    // Tests for Cordons
    #[test]
    fn cordons_try_from_valid_block() {
        let mut key_values = IndexMap::new();
        key_values.insert("active".to_string(), "1".to_string());

        let mut block = VmfBlock {
            name: "cordons".to_string(),
            key_values,
            blocks: Vec::new(),
        };

        let cordon1 = VmfBlock {
            name: "cordon".to_string(),
            key_values: {
                let mut map = IndexMap::new();
                map.insert("name".to_string(), "test_cordon".to_string());
                map.insert("active".to_string(), "1".to_string());
                map
            },
            blocks: vec![VmfBlock {
                name: "box".to_string(),
                key_values: {
                    let mut map = IndexMap::new();
                    map.insert("mins".to_string(), "0 0 0".to_string());
                    map.insert("maxs".to_string(), "1 1 1".to_string());
                    map
                },
                blocks: Vec::new(),
            }],
        };

        let cordon2 = VmfBlock {
            name: "cordon".to_string(),
            key_values: {
                let mut map = IndexMap::new();
                map.insert("name".to_string(), "test_cordon_2".to_string());
                map.insert("active".to_string(), "0".to_string());
                map
            },
            blocks: vec![VmfBlock {
                name: "box".to_string(),
                key_values: {
                    let mut map = IndexMap::new();
                    map.insert("mins".to_string(), "2 2 2".to_string());
                    map.insert("maxs".to_string(), "3 3 3".to_string());
                    map
                },
                blocks: Vec::new(),
            }],
        };

        block.blocks.push(cordon1);
        block.blocks.push(cordon2);

        let cordons = Cordons::try_from(block).unwrap();

        assert_eq!(cordons.active, 1);
        assert_eq!(cordons.cordons.len(), 2);
        assert_eq!(cordons.cordons[0].name, "test_cordon");
        assert_eq!(cordons.cordons[0].active, true);
        assert_eq!(cordons.cordons[0].min, "0 0 0");
        assert_eq!(cordons.cordons[0].max, "1 1 1");
        assert_eq!(cordons.cordons[1].name, "test_cordon_2");
        assert_eq!(cordons.cordons[1].active, false);
        assert_eq!(cordons.cordons[1].min, "2 2 2");
        assert_eq!(cordons.cordons[1].max, "3 3 3");
    }

    #[test]
    fn cordons_try_from_missing_key() {
        let block = VmfBlock {
            name: "cordons".to_string(),
            key_values: IndexMap::new(),
            blocks: Vec::new(),
        };

        let result = Cordons::try_from(block);

        assert!(matches!(result, Err(VmfError::InvalidFormat(_))));
    }

    #[test]
    fn cordons_try_from_invalid_type() {
        let mut key_values = IndexMap::new();
        key_values.insert("active".to_string(), "abc".to_string());

        let block = VmfBlock {
            name: "cordons".to_string(),
            key_values,
            blocks: Vec::new(),
        };

        let result = Cordons::try_from(block);
        assert!(matches!(
            result,
            Err(VmfError::ParseInt { source: _, key: _ })
        ));
    }

    #[test]
    fn cordons_to_vmf_string() {
        let cordons = Cordons {
            active: 1,
            cordons: vec![
                Cordon {
                    name: "test_cordon".to_string(),
                    active: true,
                    min: "0 0 0".to_string(),
                    max: "1 1 1".to_string(),
                },
                Cordon {
                    name: "test_cordon_2".to_string(),
                    active: false,
                    min: "2 2 2".to_string(),
                    max: "3 3 3".to_string(),
                },
            ],
        };
        let expected = "\
        cordons\n\
        {\n\
        \t\"active\" \"1\"\n\
        \tcordon\n\
        \t{\n\
        \t\t\"name\" \"test_cordon\"\n\
        \t\t\"active\" \"1\"\n\
        \t\tbox\n\
        \t\t{\n\
        \t\t\t\"mins\" \"0 0 0\"\n\
        \t\t\t\"maxs\" \"1 1 1\"\n\
        \t\t}\n\
        \t}\n\
        \tcordon\n\
        \t{\n\
        \t\t\"name\" \"test_cordon_2\"\n\
        \t\t\"active\" \"0\"\n\
        \t\tbox\n\
        \t\t{\n\
        \t\t\t\"mins\" \"2 2 2\"\n\
        \t\t\t\"maxs\" \"3 3 3\"\n\
        \t\t}\n\
        \t}\n\
        }\n";

        assert_eq!(cordons.to_vmf_string(0), expected);
    }

    #[test]
    fn cordons_into_vmf_block() {
        let cordons = Cordons {
            active: 1,
            cordons: vec![
                Cordon {
                    name: "test_cordon".to_string(),
                    active: true,
                    min: "0 0 0".to_string(),
                    max: "1 1 1".to_string(),
                },
                Cordon {
                    name: "test_cordon_2".to_string(),
                    active: false,
                    min: "2 2 2".to_string(),
                    max: "3 3 3".to_string(),
                },
            ],
        };
        let block: VmfBlock = cordons.into();

        assert_eq!(block.name, "cordons");
        assert_eq!(block.key_values.get("active"), Some(&"1".to_string()));
        assert_eq!(block.blocks.len(), 2);
    }

    // Tests for Cordon
    #[test]
    fn cordon_try_from_valid_block() {
        let mut key_values = IndexMap::new();
        key_values.insert("name".to_string(), "test_cordon".to_string());
        key_values.insert("active".to_string(), "1".to_string());

        let block = VmfBlock {
            name: "cordon".to_string(),
            key_values,
            blocks: vec![VmfBlock {
                name: "box".to_string(),
                key_values: {
                    let mut map = IndexMap::new();
                    map.insert("mins".to_string(), "0 0 0".to_string());
                    map.insert("maxs".to_string(), "1 1 1".to_string());
                    map
                },
                blocks: Vec::new(),
            }],
        };

        let cordon = Cordon::try_from(block).unwrap();

        assert_eq!(cordon.name, "test_cordon");
        assert_eq!(cordon.active, true);
        assert_eq!(cordon.min, "0 0 0");
        assert_eq!(cordon.max, "1 1 1");
    }

    #[test]
    fn cordon_try_from_missing_box_block() {
        let mut key_values = IndexMap::new();
        key_values.insert("name".to_string(), "test_cordon".to_string());
        key_values.insert("active".to_string(), "1".to_string());

        let block = VmfBlock {
            name: "cordon".to_string(),
            key_values,
            blocks: Vec::new(),
        };

        let result = Cordon::try_from(block);

        assert!(matches!(result, Err(VmfError::InvalidFormat(_))));
    }

    #[test]
    fn cordon_try_from_missing_key() {
        let mut key_values = IndexMap::new();
        key_values.insert("name".to_string(), "test_cordon".to_string());

        let block = VmfBlock {
            name: "cordon".to_string(),
            key_values,
            blocks: vec![VmfBlock {
                name: "box".to_string(),
                key_values: {
                    let mut map = IndexMap::new();
                    map.insert("mins".to_string(), "0 0 0".to_string());
                    map.insert("maxs".to_string(), "1 1 1".to_string());
                    map
                },
                blocks: Vec::new(),
            }],
        };

        let result = Cordon::try_from(block);

        assert!(matches!(result, Err(VmfError::InvalidFormat(_))));
    }

    #[test]
    fn cordon_to_vmf_string() {
        let cordon = Cordon {
            name: "test_cordon".to_string(),
            active: true,
            min: "0 0 0".to_string(),
            max: "1 1 1".to_string(),
        };
        let expected = "\
        cordon\n\
        {\n\
        \t\"name\" \"test_cordon\"\n\
        \t\"active\" \"1\"\n\
        \tbox\n\
        \t{\n\
        \t\t\"mins\" \"0 0 0\"\n\
        \t\t\"maxs\" \"1 1 1\"\n\
        \t}\n\
        }\n";
        assert_eq!(cordon.to_vmf_string(0), expected);
    }

    #[test]
    fn cordon_into_vmf_block() {
        let cordon = Cordon {
            name: "test_cordon".to_string(),
            active: true,
            min: "0 0 0".to_string(),
            max: "1 1 1".to_string(),
        };
        let block: VmfBlock = cordon.into();
        assert_eq!(block.name, "cordon");
        assert_eq!(
            block.key_values.get("name"),
            Some(&"test_cordon".to_string())
        );
        assert_eq!(block.key_values.get("active"), Some(&"1".to_string()));
        assert_eq!(block.blocks.len(), 1);
        assert_eq!(block.blocks[0].name, "box");
    }
}
