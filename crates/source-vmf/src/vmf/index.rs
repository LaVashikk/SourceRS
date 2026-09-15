//! Entity addressing and targetname index for fast lookups

use std::collections::HashMap;

use super::entities::Entity;
use super::file::VmfFile;

/// Addresses an entity in a [`VmfFile`] across visible and hidden entities.
///
/// Valid only while entities are not inserted, removed, or reordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntId {
    /// Whether the entity is in [`VmfFile::hiddens`].
    pub hidden: bool,
    pub idx: u32,
}

impl VmfFile {
    /// Returns a reference to the entity at `id`, or `None` if the index is out of bounds.
    pub fn entity(&self, id: EntId) -> Option<&Entity> {
        let list = if id.hidden {
            &self.hiddens
        } else {
            &self.entities
        };
        list.get(id.idx as usize)
    }

    /// Returns a mutable reference to the entity at `id`, or `None` if the index is out of bounds.
    pub fn entity_mut(&mut self, id: EntId) -> Option<&mut Entity> {
        let list = if id.hidden {
            &mut self.hiddens
        } else {
            &mut self.entities
        };
        list.get_mut(id.idx as usize)
    }

    /// Iterates over all entities in the file, including hidden ones.
    pub fn iter_entities(&self) -> impl Iterator<Item = (EntId, &Entity)> {
        let visible = self.entities.iter().enumerate().map(|(i, ent)| {
            (
                EntId {
                    hidden: false,
                    idx: i as u32,
                },
                ent,
            )
        });
        let hidden = self.hiddens.iter().enumerate().map(|(i, ent)| {
            (
                EntId {
                    hidden: true,
                    idx: i as u32,
                },
                ent,
            )
        });
        visible.chain(hidden)
    }

    /// Mutably iterates over all entities in the file, including hidden ones.
    pub fn iter_entities_mut(&mut self) -> impl Iterator<Item = (EntId, &mut Entity)> {
        let visible = self.entities.iter_mut().enumerate().map(|(i, ent)| {
            (
                EntId {
                    hidden: false,
                    idx: i as u32,
                },
                ent,
            )
        });
        let hidden = self.hiddens.iter_mut().enumerate().map(|(i, ent)| {
            (
                EntId {
                    hidden: true,
                    idx: i as u32,
                },
                ent,
            )
        });
        visible.chain(hidden)
    }
}

/// Targetname index mapping entity names to their identifiers.
///
/// Names are not unique in a VMF, so lookups return slices of [`EntId`].
#[derive(Debug, Default, Clone)]
pub struct EntityIndex {
    names: HashMap<Box<str>, Vec<EntId>>,
}

impl EntityIndex {
    /// Builds an index from the given [`VmfFile`].
    pub fn build(vmf: &VmfFile) -> Self {
        let mut names: HashMap<Box<str>, Vec<EntId>> = HashMap::new();
        for (id, ent) in vmf.iter_entities() {
            if let Some(name) = ent.targetname() {
                names.entry(name.into()).or_default().push(id);
            }
        }
        Self { names }
    }

    /// Returns entity IDs matching the given targetname exactly.
    pub fn by_name(&self, name: &str) -> &[EntId] {
        self.names.get(name).map_or(&[], |ids| ids.as_slice())
    }

    /// Returns entity IDs whose targetname starts with `prefix`.
    pub fn by_prefix<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = EntId> + 'a {
        self.names
            .iter()
            .filter(move |(name, _)| name.starts_with(prefix))
            .flat_map(|(_, ids)| ids.iter().copied())
    }
}
