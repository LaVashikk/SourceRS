use std::collections::HashSet;

use super::VmfFile;
use crate::prelude::{Entity, Solid, VisGroup};

impl VmfFile {
    /// Returns an iterator over entities (including hidden ones) belonging to the specified VisGroup ID.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The ID of the target VisGroup.
    /// * `include_children` - If true, includes entities from all child VisGroups recursively.
    ///
    /// # Returns
    ///
    /// An `Option` containing an iterator yielding references to the matching `Entity` objects.
    /// Returns `None` if no VisGroup with the given `group_id` is found.
    pub fn get_entities_in_visgroup(
        &self,
        group_id: i32,
        include_children: bool,
    ) -> Option<impl Iterator<Item = &Entity> + '_> {
        // 1. Find the starting VisGroup by ID. Returns None if not found.
        let start_group = self.visgroups.find_by_id(group_id)?;

        // 2. Collect all relevant VisGroup IDs.
        let ids_to_check: HashSet<i32> = if include_children {
            let mut ids = HashSet::new();
            collect_child_visgroup_ids(start_group, &mut ids);
            ids
        } else {
            // Only the ID of the found group.
            HashSet::from([start_group.id])
        };

        // 3. Create the filtered iterator over entities and hidden entities.
        let iterator = self
            .entities
            .iter()
            .chain(self.hiddens.iter())
            .filter(move |entity| {
                // Check if the entity belongs to one of the target VisGroup IDs.
                entity
                    .editor
                    .visgroup_id.is_some_and(|ent_group_id| ids_to_check.contains(&ent_group_id))
            });

        // 4. Return the iterator wrapped in Some.
        Some(iterator)
    }

    /// Returns a mutable iterator over entities (including hidden ones) belonging to the specified VisGroup ID.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The ID of the target VisGroup.
    /// * `include_children` - If true, includes entities from all child VisGroups recursively.
    ///
    /// # Returns
    ///
    /// An `Option` containing an iterator yielding mutable references to the matching `Entity` objects.
    /// Returns `None` if no VisGroup with the given `group_id` is found.
    pub fn get_entities_in_visgroup_mut(
        &mut self,
        group_id: i32,
        include_children: bool,
    ) -> Option<impl Iterator<Item = &mut Entity> + '_> {
        // Note: returns mutable references

        // 1. Find the starting VisGroup by ID (immutable lookup is sufficient to get IDs).
        let start_group = self.visgroups.find_by_id(group_id)?;

        // 2. Collect all relevant VisGroup IDs (immutable collection is fine).
        let ids_to_check: HashSet<i32> = if include_children {
            let mut ids = HashSet::new();
            collect_child_visgroup_ids(start_group, &mut ids);
            ids
        } else {
            HashSet::from([start_group.id])
        };

        // 3. Create the filtered *mutable* iterator.
        let iterator = self
            .entities
            .iter_mut() // Get mutable iterator
            .chain(self.hiddens.iter_mut()) // Chain mutable iterator
            .filter(move |entity| {
                entity
                    .editor
                    .visgroup_id.is_some_and(|ent_group_id| ids_to_check.contains(&ent_group_id))
            });

        // 4. Return the mutable iterator wrapped in Some.
        Some(iterator)
    }

    /// Returns an iterator over world solids (visible and hidden) belonging to the specified VisGroup ID.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The ID of the target VisGroup.
    /// * `include_children` - If true, includes solids from all child VisGroups recursively.
    ///
    /// # Returns
    ///
    /// An `Option` containing an iterator yielding references to the matching `Solid` objects.
    /// Returns `None` if no VisGroup with the given `group_id` is found.
    pub fn get_solids_in_visgroup(
        &self,
        group_id: i32,
        include_children: bool,
    ) -> Option<impl Iterator<Item = &Solid> + '_> {
        // 1. Find the starting VisGroup.
        let start_group = self.visgroups.find_by_id(group_id)?;

        // 2. Collect relevant IDs.
        let ids_to_check: HashSet<i32> = if include_children {
            let mut ids = HashSet::new();
            collect_child_visgroup_ids(start_group, &mut ids);
            ids
        } else {
            HashSet::from([start_group.id])
        };

        // 3. Create filtered iterator over world solids.
        let iterator = self
            .world
            .solids
            .iter()
            .chain(self.world.hidden.iter())
            .filter(move |solid| {
                solid.editor.visgroup_id.is_some_and(|solid_group_id| {
                    ids_to_check.contains(&solid_group_id)
                })
            });

        // 4. Return iterator.
        Some(iterator)
    }

    /// Returns a mutable iterator over world solids (visible and hidden) belonging to the specified VisGroup ID.
    ///
    /// # Arguments
    ///
    /// * `group_id` - The ID of the target VisGroup.
    /// * `include_children` - If true, includes solids from all child VisGroups recursively.
    ///
    /// # Returns
    ///
    /// An `Option` containing an iterator yielding mutable references to the matching `Solid` objects.
    /// Returns `None` if no VisGroup with the given `group_id` is found.
    pub fn get_solids_in_visgroup_mut(
        &mut self,
        group_id: i32,
        include_children: bool,
    ) -> Option<impl Iterator<Item = &mut Solid> + '_> {
        // 1. Find the starting VisGroup.
        let start_group = self.visgroups.find_by_id(group_id)?;

        // 2. Collect relevant IDs.
        let ids_to_check: HashSet<i32> = if include_children {
            let mut ids = HashSet::new();
            collect_child_visgroup_ids(start_group, &mut ids);
            ids
        } else {
            HashSet::from([start_group.id])
        };

        // 3. Create filtered *mutable* iterator over world solids.
        let iterator = self
            .world
            .solids
            .iter_mut() // Mutable iterator
            .chain(self.world.hidden.iter_mut()) // Chain mutable iterator
            .filter(move |solid| {
                solid.editor.visgroup_id.is_some_and(|solid_group_id| {
                    ids_to_check.contains(&solid_group_id)
                })
            });

        // 4. Return mutable iterator.
        Some(iterator)
    }
}

/// Recursively collects the IDs of a VisGroup and all its children into a HashSet.
/// The passed `group` must be the one found by ID/Name previously.
/// Uses the `collected_ids` set to avoid infinite loops in case of (unlikely) cycles.
fn collect_child_visgroup_ids(group: &VisGroup, collected_ids: &mut HashSet<i32>) {
    // Insert the current group's ID. If it was already present, stop to prevent cycles.
    if !collected_ids.insert(group.id) {
        return;
    }

    // Recursively collect IDs from children
    if let Some(ref children) = group.children {
        for child in children {
            collect_child_visgroup_ids(child, collected_ids);
        }
    }
}
