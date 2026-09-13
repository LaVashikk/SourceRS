use std::collections::HashSet;
use vmf_forge::prelude::*;

// Helper function to create a VmfFile with a predefined VisGroup hierarchy and objects
fn create_test_vmf() -> VmfFile {
    let mut vmf = VmfFile::default();

    // --- VisGroup Hierarchy ---
    //  1: Parent
    //  |-- 2: Child1
    //  |   |-- 4: Grandchild
    //  3: Parent2 (no children)
    //  5: Other (no children)
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
    let other = VisGroup {
        id: 5,
        name: "Other".to_string(),
        color: "0 255 255".to_string(),
        children: None,
    };

    vmf.visgroups.groups = vec![parent1, parent2, other];

    // --- Entities ---
    let mut ent_no_group = Entity::new("ent_no_group", 100);
    ent_no_group.editor.visgroup_id = None; // Explicitly None

    let mut ent_parent = Entity::new("ent_parent", 101);
    ent_parent.editor.visgroup_id = Some(1);

    let mut ent_child1 = Entity::new("ent_child1", 102);
    ent_child1.editor.visgroup_id = Some(2);

    let mut ent_grandchild = Entity::new("ent_grandchild", 104);
    ent_grandchild.editor.visgroup_id = Some(4);

    let mut ent_other = Entity::new("ent_other", 105);
    ent_other.editor.visgroup_id = Some(5);

    let mut hidden_ent_parent = Entity::new("hidden_ent_parent", 201);
    hidden_ent_parent.editor.visgroup_id = Some(1);
    hidden_ent_parent.is_hidden = true; // Mark as conceptually hidden

    vmf.entities.push(ent_no_group);
    vmf.entities.push(ent_parent);
    vmf.entities.push(ent_child1);
    vmf.entities.push(ent_grandchild);
    vmf.entities.push(ent_other);
    vmf.hiddens.push(hidden_ent_parent); // Add to hiddens list

    // --- Solids ---
    let mut solid_no_group = Solid {
        id: 500,
        ..Default::default()
    };
    solid_no_group.editor.visgroup_id = None;

    let mut solid_parent = Solid {
        id: 501,
        ..Default::default()
    };
    solid_parent.editor.visgroup_id = Some(1);
    solid_parent.editor.color = "255 0 0".to_string(); // For mut test

    let mut solid_child1 = Solid {
        id: 502,
        ..Default::default()
    };
    solid_child1.editor.visgroup_id = Some(2);
    solid_child1.editor.color = "0 255 0".to_string(); // For mut test

    let mut hidden_solid_child1 = Solid {
        id: 602,
        ..Default::default()
    };
    hidden_solid_child1.editor.visgroup_id = Some(2);

    vmf.world.solids.push(solid_no_group);
    vmf.world.solids.push(solid_parent);
    vmf.world.solids.push(solid_child1);
    vmf.world.hidden.push(hidden_solid_child1); // Add to world.hidden list

    vmf
}

#[cfg(test)]
mod tests {
    use super::*;
    // --- Tests for get_entities_in_visgroup (immutable) ---
    #[test]
    fn test_get_entities_direct_no_children() {
        let vmf = create_test_vmf();
        let entities_iter = vmf.get_entities_in_visgroup(1, false); // Get Parent (ID 1), no children
        assert!(entities_iter.is_some());
        let entity_ids: HashSet<u64> = entities_iter.unwrap().map(|e| e.id()).collect();
        // Should contain ent_parent (101) and hidden_ent_parent (201)
        assert_eq!(entity_ids, HashSet::from([101, 201]));
    }

    #[test]
    fn test_get_entities_child_no_children() {
        let vmf = create_test_vmf();
        let entities_iter = vmf.get_entities_in_visgroup(2, false); // Get Child1 (ID 2), no children
        assert!(entities_iter.is_some());
        let entity_ids: HashSet<u64> = entities_iter.unwrap().map(|e| e.id()).collect();
        assert_eq!(entity_ids, HashSet::from([102]));
    }

    #[test]
    fn test_get_entities_grandchild_no_children() {
        let vmf = create_test_vmf();
        let entities_iter = vmf.get_entities_in_visgroup(4, false); // Get Grandchild (ID 4), no children
        assert!(entities_iter.is_some());
        let entity_ids: HashSet<u64> = entities_iter.unwrap().map(|e| e.id()).collect();
        assert_eq!(entity_ids, HashSet::from([104]));
    }

    #[test]
    fn test_get_entities_parent_with_children() {
        let vmf = create_test_vmf();
        let entities_iter = vmf.get_entities_in_visgroup(1, true); // Get Parent (ID 1), WITH children
        assert!(entities_iter.is_some());
        let entity_ids: HashSet<u64> = entities_iter.unwrap().map(|e| e.id()).collect();
        assert_eq!(entity_ids, HashSet::from([101, 201, 102, 104]));
    }

    #[test]
    fn test_get_entities_child_with_children() {
        let vmf = create_test_vmf();
        let entities_iter = vmf.get_entities_in_visgroup(2, true); // Get Child1 (ID 2), WITH children
        assert!(entities_iter.is_some());
        let entity_ids: HashSet<u64> = entities_iter.unwrap().map(|e| e.id()).collect();
        assert_eq!(entity_ids, HashSet::from([102, 104]));
    }

    #[test]
    fn test_get_entities_no_entities_in_group() {
        let vmf = create_test_vmf();
        // Group 3 exists but has no entities assigned
        let entities_iter_no_children = vmf.get_entities_in_visgroup(3, false);
        assert!(entities_iter_no_children.is_some());
        assert_eq!(entities_iter_no_children.unwrap().count(), 0);

        let entities_iter_with_children = vmf.get_entities_in_visgroup(3, true);
        assert!(entities_iter_with_children.is_some());
        assert_eq!(entities_iter_with_children.unwrap().count(), 0);
    }

    #[test]
    fn test_get_entities_group_not_found() {
        let vmf = create_test_vmf();
        let entities_iter = vmf.get_entities_in_visgroup(99, false); // Non-existent ID
        assert!(entities_iter.is_none());
        let entities_iter_children = vmf.get_entities_in_visgroup(99, true); // Non-existent ID
        assert!(entities_iter_children.is_none());
    }

    // --- Tests for get_solids_in_visgroup (immutable) ---
    #[test]
    fn test_get_solids_direct_no_children() {
        let vmf = create_test_vmf();
        let solids_iter = vmf.get_solids_in_visgroup(1, false); // Get Parent (ID 1), no children
        assert!(solids_iter.is_some());
        let solid_ids: HashSet<u64> = solids_iter.unwrap().map(|s| s.id).collect();
        // Should contain solid_parent (501)
        assert_eq!(solid_ids, HashSet::from([501]));
    }

    #[test]
    fn test_get_solids_child_no_children() {
        let vmf = create_test_vmf();
        let solids_iter = vmf.get_solids_in_visgroup(2, false); // Get Child1 (ID 2), no children
        assert!(solids_iter.is_some());
        let solid_ids: HashSet<u64> = solids_iter.unwrap().map(|s| s.id).collect();
        assert_eq!(solid_ids, HashSet::from([502, 602]));
    }

    #[test]
    fn test_get_solids_parent_with_children() {
        let vmf = create_test_vmf();
        let solids_iter = vmf.get_solids_in_visgroup(1, true); // Get Parent (ID 1), WITH children
        assert!(solids_iter.is_some());
        let solid_ids: HashSet<u64> = solids_iter.unwrap().map(|s| s.id).collect();
        assert_eq!(solid_ids, HashSet::from([501, 502, 602]));
    }

    #[test]
    fn test_get_solids_no_solids_in_group() {
        let vmf = create_test_vmf();
        // Group 4 exists but has no solids assigned
        let solids_iter = vmf.get_solids_in_visgroup(4, false);
        assert!(solids_iter.is_some());
        assert_eq!(solids_iter.unwrap().count(), 0);

        let solids_iter_children = vmf.get_solids_in_visgroup(4, true);
        assert!(solids_iter_children.is_some());
        assert_eq!(solids_iter_children.unwrap().count(), 0);
    }

    #[test]
    fn test_get_solids_group_not_found() {
        let vmf = create_test_vmf();
        let solids_iter = vmf.get_solids_in_visgroup(99, false); // Non-existent ID
        assert!(solids_iter.is_none());
        let solids_iter_children = vmf.get_solids_in_visgroup(99, true); // Non-existent ID
        assert!(solids_iter_children.is_none());
    }

    // --- Tests for get_entities_in_visgroup_mut (mutable) ---
    #[test]
    fn test_get_entities_mut_modification() {
        let mut vmf = create_test_vmf();
        let new_key = "modified_key".to_string();
        let new_value = "modified_value".to_string();
        let target_entity_id = 101; // ent_parent in group 1

        // Scope the mutable borrow
        {
            let entities_iter_mut = vmf.get_entities_in_visgroup_mut(1, false);
            assert!(entities_iter_mut.is_some());
            let mut found = false;
            for entity in entities_iter_mut.unwrap() {
                if entity.id() == target_entity_id {
                    entity.set(new_key.clone(), new_value.clone());
                    found = true;
                    break;
                }
            }
            assert!(
                found,
                "Target entity {} not found in iterator",
                target_entity_id
            );
        } // Mutable borrow ends here

        // Verify the change outside the borrow
        let modified_entity = vmf.entities.iter().find(|e| e.id() == target_entity_id);
        assert!(modified_entity.is_some());
        assert_eq!(modified_entity.unwrap().get(&new_key), Some(&new_value));
    }

    #[test]
    fn test_get_entities_mut_group_not_found() {
        let mut vmf = create_test_vmf();
        let entities_iter_mut = vmf.get_entities_in_visgroup_mut(99, false);
        assert!(entities_iter_mut.is_none());
    }

    // --- Tests for get_solids_in_visgroup_mut (mutable) ---
    #[test]
    fn test_get_solids_mut_modification() {
        let mut vmf = create_test_vmf();
        let new_color = "1 2 3".to_string();
        let target_solid_id = 502; // solid_child1 in group 2

        // Scope the mutable borrow
        {
            let solids_iter_mut = vmf.get_solids_in_visgroup_mut(2, true); // Include children (though not needed for ID 502)
            assert!(solids_iter_mut.is_some());
            let mut found = false;
            for solid in solids_iter_mut.unwrap() {
                if solid.id == target_solid_id {
                    solid.editor.color = new_color.clone();
                    found = true;
                    break;
                }
            }
            assert!(
                found,
                "Target solid {} not found in iterator",
                target_solid_id
            );
        } // Mutable borrow ends here

        // Verify the change outside the borrow
        let modified_solid = vmf.world.solids.iter().find(|s| s.id == target_solid_id);
        assert!(modified_solid.is_some());
        assert_eq!(modified_solid.unwrap().editor.color, new_color);
    }

    #[test]
    fn test_get_solids_mut_group_not_found() {
        let mut vmf = create_test_vmf();
        let solids_iter_mut = vmf.get_solids_in_visgroup_mut(99, false);
        assert!(solids_iter_mut.is_none());
    }
}
