#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use vmf_forge::VmfFile;
    use vmf_forge::errors::VmfError;

    #[test]
    fn open_and_parse_valid_vmf() {
        let vmf_file = VmfFile::open("vmf_examples/valid.vmf").unwrap();

        assert_eq!(vmf_file.versioninfo.editor_version, 400);
        assert_eq!(vmf_file.versioninfo.editor_build, 9672);
        assert_eq!(vmf_file.versioninfo.map_version, 2);
        assert_eq!(vmf_file.versioninfo.format_version, 100);
        assert_eq!(vmf_file.versioninfo.prefab, false);
        assert_eq!(vmf_file.world.key_values.get("id").unwrap(), "1");
        assert_eq!(
            vmf_file.world.key_values.get("classname").unwrap(),
            "worldspawn"
        );
    }

    #[test]
    fn open_and_parse_empty_vmf() {
        let vmf_file = VmfFile::open("vmf_examples/empty.vmf").unwrap();

        assert_eq!(vmf_file.versioninfo.editor_version, 0);
        assert_eq!(vmf_file.versioninfo.editor_build, 0);
        assert_eq!(vmf_file.versioninfo.map_version, 0);
        assert_eq!(vmf_file.versioninfo.format_version, 0);
        assert_eq!(vmf_file.versioninfo.prefab, false);
        assert!(vmf_file.world.key_values.is_empty());
        assert!(vmf_file.entities.is_empty());
        assert!(vmf_file.cameras.cams.is_empty());
    }

    #[test]
    fn open_and_parse_invalid_vmf() {
        let result = VmfFile::open("vmf_examples/invalid.vmf");

        assert!(matches!(result, Err(VmfError::Parse(_))));
    }

    #[test]
    fn open_and_parse_nonexistent_vmf() {
        let result = VmfFile::open("vmf_examples/nonexistent.vmf");
        assert!(matches!(result, Err(VmfError::Io(_))));
    }

    #[test]
    fn to_vmf_string_matches_original() {
        let input = "\
        versioninfo\n\
        {\n\
        \t\"editorversion\" \"400\"\n\
        \t\"editorbuild\" \"8000\"\n\
        \t\"mapversion\" \"1\"\n\
        \t\"formatversion\" \"100\"\n\
        \t\"prefab\" \"0\"\n\
        }\n\
        visgroups\n\
        {\n\
        }\n\
        viewsettings\n\
        {\n\
        \t\"bSnapToGrid\" \"0\"\n\
        \t\"bShowGrid\" \"0\"\n\
        \t\"bShowLogicalGrid\" \"0\"\n\
        \t\"nGridSpacing\" \"0\"\n\
        \t\"bShow3DGrid\" \"0\"\n\
        }\n\
        world\n\
        {\n\
        \t\"id\" \"1\"\n\
        \t\"classname\" \"worldspawn\"\n\
        \tgroup\n\
        \t{\n\
        \t\t\"id\" \"0\"\n\
        \t\teditor\n\
        \t\t{\n\
        \t\t\t\"color\" \"255 255 255\"\n\
        \t\t\t\"visgroupshown\" \"1\"\n\
        \t\t\t\"visgroupautoshown\" \"1\"\n\
        \t\t}\n\
        \t}\n\
        }\n\
        cameras\n\
        {\n\
        \t\"activecamera\" \"0\"\n\
        }\n\
        cordons\n\
        {\n\
        \t\"active\" \"0\"\n\
        }\n";

        let vmf_file = VmfFile::parse(input).unwrap();
        let output = vmf_file.to_vmf_string();
        assert_eq!(output, input);
    }

    #[test]
    fn open_and_parse_complex_vmf() {
        // Assuming you have a complex.vmf file in your vmf_examples folder
        let vmf_file = VmfFile::open("vmf_examples/complex.vmf").unwrap();

        // Assertions to check specific elements of the complex VMF
        assert!(!vmf_file.entities.is_empty());
        assert!(!vmf_file.world.solids.is_empty());
        assert!(!vmf_file.visgroups.groups.is_empty());
        assert!(!vmf_file.cameras.cams.is_empty());
        assert!(vmf_file.cordons.cordons.is_empty());

        // Assert a few specific entities
        assert!(
            vmf_file
                .entities
                .iter()
                .any(|e| e.key_values.get("classname") == Some(&"trigger_multiple".to_string()))
        );
        assert!(
            vmf_file
                .entities
                .iter()
                .any(|e| e.key_values.get("targetname") == Some(&"door_0".to_string()))
        );

        assert!(vmf_file.world.solids.iter().any(|s| s.id == 9157));
        assert_eq!(vmf_file.visgroups.groups.len(), 1);
        assert_eq!(vmf_file.cameras.cams.len(), 1);
    }

    #[test]
    fn parse_vmf_with_many_entities() {
        // Create a string with a VMF containing many entities
        let mut input = String::from(
            "versioninfo\n{\n\t\"editorversion\" \"400\"\n\t\"editorbuild\" \"8000\"\n\t\"mapversion\" \"1\"\n\t\"formatversion\" \"100\"\n\t\"prefab\" \"0\"\n}\nworld\n{\n\t\"id\" \"1\"\n\t\"classname\" \"worldspawn\"\n}\n",
        );
        for i in 0..1000 {
            input.push_str(&format!("entity\n{{\n\t\"id\" \"{}\"\n\t\"classname\" \"info_player_start\"\n\t\"origin\" \"0 0 0\"\n}}\n", i + 2));
        }

        let vmf_file = VmfFile::parse(&input).unwrap();
        assert_eq!(vmf_file.entities.len(), 1000);
    }

    #[test]
    fn parse_vmf_with_hidden_blocks() {
        let input = "\
        versioninfo\n\
        {\n\
        \t\"editorversion\" \"400\"\n\
        \t\"editorbuild\" \"8000\"\n\
        \t\"mapversion\" \"1\"\n\
        \t\"formatversion\" \"100\"\n\
        \t\"prefab\" \"0\"\n\
        }\n\
        world\n\
        {\n\
        \t\"id\" \"1\"\n\
        \t\"classname\" \"worldspawn\"\n\
        }\n\
        entity\n\
        {\n\
        \t\"id\" \"2\"\n\
        \t\"classname\" \"func_detail\"\n\
        \thidden\n\
        \t{\n\
        \t\tsolid\n\
        \t\t{\n\
        \t\t\t\"id\" \"3\"\n\
        \t\t}\n\
        \t}\n\
        }\n";
        let vmf_file = VmfFile::parse(input).unwrap();

        assert_eq!(vmf_file.entities.len(), 1);
        assert_eq!(vmf_file.hiddens.len(), 0); // We don't track the hidden solids in the VmfFile struct
        let entity = &vmf_file.entities[0];
        assert!(entity.solids.is_some());
        assert_eq!(entity.solids.as_ref().unwrap().len(), 1);
        assert_eq!(entity.solids.as_ref().unwrap()[0].id, 3);
    }

    #[test]
    fn parse_vmf_with_cordons() {
        let input = "\
        versioninfo\n\
        {\n\
        \t\"editorversion\" \"400\"\n\
        \t\"editorbuild\" \"8000\"\n\
        \t\"mapversion\" \"1\"\n\
        \t\"formatversion\" \"100\"\n\
        \t\"prefab\" \"0\"\n\
        }\n\
        cordons\n\
        {\n\
        \t\"active\" \"1\"\n\
        \tcordon\n\
        \t{\n\
        \t\t\"name\" \"cordon_1\"\n\
        \t\t\"active\" \"1\"\n\
        \t\tbox\n\
        \t\t{\n\
        \t\t\t\"mins\" \"(-64 -64 -64)\"\n\
        \t\t\t\"maxs\" \"(64 64 64)\"\n\
        \t\t}\n\
        \t}\n\
        }\n";

        let vmf_file = VmfFile::parse(input).unwrap();

        assert_eq!(vmf_file.cordons.active, 1);
        assert_eq!(vmf_file.cordons.cordons.len(), 1);
        assert_eq!(vmf_file.cordons.cordons[0].name, "cordon_1");
        assert_eq!(vmf_file.cordons.cordons[0].active, true);
        assert_eq!(vmf_file.cordons.cordons[0].min, "(-64 -64 -64)");
        assert_eq!(vmf_file.cordons.cordons[0].max, "(64 64 64)");
    }

    #[test]
    fn parse_vmf_with_cameras() {
        let input = "\
        versioninfo\n\
        {\n\
            \"editorversion\" \"400\"\n\
            \"editorbuild\" \"8000\"\n\
            \"mapversion\" \"1\"\n\
            \"formatversion\" \"100\"\n\
            \"prefab\" \"0\"\n\
        }\n\
        cameras\n\
        {\n\
            \"activecamera\" \"1\"\n\
            camera\n\
            {\n\
                \"position\" \"[0 0 0]\"\n\
                \"look\" \"[1 0 0]\"\n\
            }\n\
            camera\n\
            {\n\
                \"position\" \"[0 1 0]\"\n\
                \"look\" \"[0 1 1]\"\n\
            }\n\
        }\n";

        let vmf_file = VmfFile::parse(input).unwrap();

        assert_eq!(vmf_file.cameras.active, 1);
        assert_eq!(vmf_file.cameras.cams.len(), 2);
        assert_eq!(vmf_file.cameras.cams[0].position, "[0 0 0]");
        assert_eq!(vmf_file.cameras.cams[0].look, "[1 0 0]");
        assert_eq!(vmf_file.cameras.cams[1].position, "[0 1 0]");
        assert_eq!(vmf_file.cameras.cams[1].look, "[0 1 1]");
    }

    #[test]
    fn to_vmf_string_complex_vmf() {
        let vmf_file = VmfFile::open("vmf_examples/complex.vmf").unwrap();
        let input = std::fs::read_to_string("vmf_examples/complex.vmf").unwrap();
        let output = vmf_file.to_vmf_string();
        assert_eq!(output, input);
    }

    #[test]
    fn to_vmf_string_valid_vmf() {
        let vmf_file = VmfFile::open("vmf_examples/valid.vmf").unwrap();
        let input = std::fs::read_to_string("vmf_examples/valid.vmf").unwrap();
        let output = vmf_file.to_vmf_string();
        assert_eq!(output, input);
    }
}
