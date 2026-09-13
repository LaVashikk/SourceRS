#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use pretty_assertions::assert_eq;
    use vmf_forge::VmfBlock;
    use vmf_forge::VmfSerializable;
    use vmf_forge::errors::VmfError;
    use vmf_forge::vmf::metadata::*;

    // Tests for VersionInfo
    #[test]
    fn version_info_try_from_valid_block() {
        let mut key_values = IndexMap::new();
        key_values.insert("editorversion".to_string(), "400".to_string());
        key_values.insert("editorbuild".to_string(), "8000".to_string());
        key_values.insert("mapversion".to_string(), "1".to_string());
        key_values.insert("formatversion".to_string(), "100".to_string());
        key_values.insert("prefab".to_string(), "0".to_string());

        let block = VmfBlock {
            name: "versioninfo".to_string(),
            key_values,
            blocks: Vec::new(),
        };

        let version_info = VersionInfo::try_from(block).unwrap();

        assert_eq!(version_info.editor_version, 400);
        assert_eq!(version_info.editor_build, 8000);
        assert_eq!(version_info.map_version, 1);
        assert_eq!(version_info.format_version, 100);
        assert_eq!(version_info.prefab, false);
    }

    #[test]
    fn version_info_try_from_missing_key() {
        let mut key_values = IndexMap::new();
        key_values.insert("editorbuild".to_string(), "8000".to_string());
        key_values.insert("mapversion".to_string(), "1".to_string());
        key_values.insert("formatversion".to_string(), "100".to_string());
        key_values.insert("prefab".to_string(), "0".to_string());

        let block = VmfBlock {
            name: "versioninfo".to_string(),
            key_values,
            blocks: Vec::new(),
        };

        let result = VersionInfo::try_from(block);

        assert!(matches!(result, Err(VmfError::InvalidFormat(_))));
    }

    #[test]
    fn version_info_try_from_invalid_type() {
        let mut key_values = IndexMap::new();
        key_values.insert("editorversion".to_string(), "400".to_string());
        key_values.insert("editorbuild".to_string(), "abc".to_string());
        key_values.insert("mapversion".to_string(), "1".to_string());
        key_values.insert("formatversion".to_string(), "100".to_string());
        key_values.insert("prefab".to_string(), "0".to_string());

        let block = VmfBlock {
            name: "versioninfo".to_string(),
            key_values,
            blocks: Vec::new(),
        };

        let result = VersionInfo::try_from(block);

        assert!(matches!(
            result,
            Err(VmfError::ParseInt { source: _, key: _ })
        ));
    }

    #[test]
    fn version_info_to_vmf_string() {
        let version_info = VersionInfo {
            editor_version: 400,
            editor_build: 8000,
            map_version: 1,
            format_version: 100,
            prefab: false,
        };

        let expected = "\
            versioninfo\n\
            {\n\
            \t\"editorversion\" \"400\"\n\
            \t\"editorbuild\" \"8000\"\n\
            \t\"mapversion\" \"1\"\n\
            \t\"formatversion\" \"100\"\n\
            \t\"prefab\" \"0\"\n\
            }\n";

        assert_eq!(version_info.to_vmf_string(0), expected);
    }

    #[test]
    fn version_info_into_vmf_block() {
        let version_info = VersionInfo {
            editor_version: 400,
            editor_build: 8000,
            map_version: 1,
            format_version: 100,
            prefab: false,
        };

        let block: VmfBlock = version_info.into();

        assert_eq!(block.name, "versioninfo");
        assert_eq!(
            block.key_values.get("editorversion"),
            Some(&"400".to_string())
        );
        assert_eq!(
            block.key_values.get("editorbuild"),
            Some(&"8000".to_string())
        );
        assert_eq!(block.key_values.get("mapversion"), Some(&"1".to_string()));
        assert_eq!(
            block.key_values.get("formatversion"),
            Some(&"100".to_string())
        );
        assert_eq!(block.key_values.get("prefab"), Some(&"0".to_string()));
        assert!(block.blocks.is_empty());
    }

    // Helper to create a test VisGroups structure
    fn create_test_visgroups() -> VisGroups {
        let grandchild = VisGroup {
            id: 4,
            name: "Grandchild".to_string(),
            color: "0 0 255".to_string(),
            children: None,
        };
        let child1 = VisGroup {
            id: 2,
            name: "Child1".to_string(),
            color: "0 255 0".to_string(),
            children: Some(vec![grandchild]),
        };
        let parent1 = VisGroup {
            id: 1,
            name: "Parent".to_string(),
            color: "255 0 0".to_string(),
            children: Some(vec![child1]),
        };
        let parent2 = VisGroup {
            id: 3,
            name: "Parent2".to_string(),
            color: "255 255 0".to_string(),
            children: None,
        };

        VisGroups {
            groups: vec![parent1, parent2],
        }
    }

    #[test]
    fn test_visgroups_find_by_id() {
        let visgroups = create_test_visgroups();

        // Find top level
        let found_parent1 = visgroups.find_by_id(1);
        assert!(found_parent1.is_some());
        assert_eq!(found_parent1.unwrap().name, "Parent");

        // Find child
        let found_child1 = visgroups.find_by_id(2);
        assert!(found_child1.is_some());
        assert_eq!(found_child1.unwrap().name, "Child1");

        // Find grandchild
        let found_grandchild = visgroups.find_by_id(4);
        assert!(found_grandchild.is_some());
        assert_eq!(found_grandchild.unwrap().name, "Grandchild");

        // Find non-existent
        let not_found = visgroups.find_by_id(99);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_visgroups_find_by_name() {
        let visgroups = create_test_visgroups();

        // Find top level
        let found_parent1 = visgroups.find_by_name("Parent");
        assert!(found_parent1.is_some());
        assert_eq!(found_parent1.unwrap().id, 1);

        // Find child
        let found_child1 = visgroups.find_by_name("Child1");
        assert!(found_child1.is_some());
        assert_eq!(found_child1.unwrap().id, 2);

        // Find grandchild
        let found_grandchild = visgroups.find_by_name("Grandchild");
        assert!(found_grandchild.is_some());
        assert_eq!(found_grandchild.unwrap().id, 4);

        // Find non-existent
        let not_found = visgroups.find_by_name("NonExistent");
        assert!(not_found.is_none());

        // Test case sensitivity (assuming names are case-sensitive)
        let case_mismatch = visgroups.find_by_name("parent");
        assert!(case_mismatch.is_none());
    }

    #[test]
    fn test_visgroups_find_by_id_mut() {
        let mut visgroups = create_test_visgroups();
        let new_color = "111 222 333".to_string();

        // Find and modify child
        let found_child1 = visgroups.find_by_id_mut(2);
        assert!(found_child1.is_some());
        found_child1.unwrap().color = new_color.clone();

        // Re-find immutably and verify
        let verified_child1 = visgroups.find_by_id(2);
        assert!(verified_child1.is_some());
        assert_eq!(verified_child1.unwrap().color, new_color);

        // Try find non-existent mutably
        let not_found = visgroups.find_by_id_mut(99);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_visgroups_find_by_name_mut() {
        let mut visgroups = create_test_visgroups();
        let new_name = "ParentModified".to_string();
        let original_id = 1;

        // Find and modify parent by name
        let found_parent1 = visgroups.find_by_name_mut("Parent");
        assert!(found_parent1.is_some());
        found_parent1.unwrap().name = new_name.clone();

        // Re-find immutably by ID and verify name change
        let verified_parent1 = visgroups.find_by_id(original_id);
        assert!(verified_parent1.is_some());
        assert_eq!(verified_parent1.unwrap().name, new_name);

        // Try finding the original name mutably (should fail)
        let not_found_old_name = visgroups.find_by_name_mut("Parent");
        assert!(not_found_old_name.is_none());

        // Try find non-existent mutably
        let not_found_new_name = visgroups.find_by_name_mut("NonExistent");
        assert!(not_found_new_name.is_none());
    }
}
