//! A map together with every VMF its `func_instance`s pull in.
//! It might be a bit of overkill, and I couldn't come up with a cleaner API.. but having this functionality is still useful

use std::path::{Path, PathBuf};

use crate::transform::Transform;
use crate::vmf::entities::Entity;
use crate::vmf::world::World;
use crate::{EntId, VmfFile, VmfResult};

/// One `func_instance` on the assembled map, nested ones included
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Instance {
    /// Index into [`VmfSystem::files`] of the VMF this instance places
    pub file: usize,
    /// Who placed this one: the parent instance (index into [`VmfSystem::instances`]) and the `func_instance`'s id in the parent's file.
    /// `None` for the root map
    pub parent: Option<(usize, EntId)>,
    /// From the space of `file` to world space, through every parent
    pub transform: Transform,
}

/// A map and every VMF its `func_instance`s pull in, each file loaded once
///
/// A file placed ten times is one entry in [`files`](Self::files) and ten in [`instances`](Self::instances):
/// edits go to the file, world positions come from the instances
///
/// ```no_run
/// use source_vmf::prelude::*;
///
/// let map = VmfSystem::open("maps/my_map.vmf")?;
/// for inst in &map.instances {
///     for ent in map.files[inst.file].entities.iter() {
///         if ent.classname() == Some("light") {
///             let [x, y, z] = inst.transform.point(Transform::from_entity(ent).origin);
///             println!("light at {x} {y} {z}");
///         }
///     }
/// }
/// # Ok::<(), VmfError>(())
/// ```
#[derive(Debug)]
pub struct VmfSystem {
    /// `files[0]` is the root map. Never shrinks, so indices survive [`relink`](Self::relink).
    /// Every [`VmfFile::path`] is canonical, that's the key files get deduplicated by
    pub files: Vec<VmfFile>,
    /// `instances[0]` is the root map. Parents always come before their children
    pub instances: Vec<Instance>,
    /// Searched after the placing VMF's directory and its `maps` directory, like gameinfo's `InstancePath`
    pub search_dirs: Vec<PathBuf>,
    /// Missing, unreadable and self-including instances found by the last [`relink`](Self::relink)
    pub warnings: Vec<String>,
}

/// The whole map in world space, made by [`VmfSystem::flatten`]
#[derive(Debug, Clone)]
pub struct Flattened {
    pub vmf: VmfFile,
    /// Where each entity of `vmf.entities` came from: index into [`VmfSystem::instances`] and its id in that instance's file
    pub sources: Vec<(usize, EntId)>,
}

impl VmfSystem {
    /// Opens a map and every VMF its `func_instance`s pull in
    pub fn open(path: impl AsRef<Path>) -> VmfResult<Self> {
        Self::open_with(path, Vec::new())
    }

    /// Like [`open`](Self::open), also looking for instance files in `search_dirs`
    pub fn open_with(path: impl AsRef<Path>, search_dirs: Vec<PathBuf>) -> VmfResult<Self> {
        let root = VmfFile::open(path.as_ref().canonicalize()?)?;
        let mut system = Self {
            files: vec![root],
            instances: Vec::new(),
            search_dirs,
            warnings: Vec::new(),
        };
        system.relink();
        Ok(system)
    }

    /// Rebuilds [`instances`](Self::instances) from the visible `func_instance`s of the files, loading the files that are not loaded yet
    ///
    /// Call it after removing entities ([`EntId`] is a position) or editing a `func_instance`'s `file`, `origin` or `angles`
    pub fn relink(&mut self) {
        self.warnings.clear();
        self.instances.clear();
        self.instances.push(Instance {
            file: 0,
            parent: None,
            transform: Transform::IDENTITY,
        });

        // `instances` doubles as the BFS queue, that's what keeps parents before their children
        let mut current = 0;
        while current < self.instances.len() {
            let vmf = &self.files[self.instances[current].file];
            let placer = vmf.path.clone().unwrap_or_default();
            let dir = Path::new(&placer).parent().map(Path::to_path_buf);
            let links: Vec<(u32, String, Transform)> = vmf
                .entities
                .iter()
                .enumerate()
                .filter(|(_, ent)| is_class(ent, "func_instance"))
                .filter_map(|(idx, ent)| {
                    Some((
                        idx as u32,
                        ent.get("file")?.clone(),
                        Transform::from_entity(ent),
                    ))
                })
                .collect();

            for (idx, name, local) in links {
                let Some(file) = self.load(&name, dir.as_deref(), &placer) else {
                    continue;
                };
                if self.chain(current).any(|i| self.instances[i].file == file) {
                    self.warnings
                        .push(format!("{placer}: instance '{name}' includes itself"));
                    continue;
                }
                let transform = self.instances[current].transform * local;
                self.instances.push(Instance {
                    file,
                    parent: Some((current, EntId { hidden: false, idx })),
                    transform,
                });
            }
            current += 1;
        }
    }

    /// `instance` followed by its parents up to the root map
    pub fn chain(&self, instance: usize) -> impl Iterator<Item = usize> {
        std::iter::successors(Some(instance), |&i| {
            self.instances[i].parent.map(|(parent, _)| parent)
        })
    }

    /// Every entity of every file, hidden ones included like [`VmfFile::iter_entities`].
    /// A file placed ten times is still visited once
    pub fn iter_entities(&self) -> impl Iterator<Item = (usize, EntId, &Entity)> {
        self.files
            .iter()
            .enumerate()
            .flat_map(|(file, vmf)| vmf.iter_entities().map(move |(id, ent)| (file, id, ent)))
    }

    /// Mutable [`iter_entities`](Self::iter_entities)
    pub fn iter_entities_mut(&mut self) -> impl Iterator<Item = (usize, EntId, &mut Entity)> {
        self.files.iter_mut().enumerate().flat_map(|(file, vmf)| {
            vmf.iter_entities_mut()
                .map(move |(id, ent)| (file, id, ent))
        })
    }

    /// Merges the root and every placed instance into one map in world space
    ///
    /// Brushes and entities go through [`Transform::apply_solid`] and [`Transform::apply_entity`].
    /// `func_instance` and `func_instance_parms` are dropped and hidden objects are left out, like VBSP does.
    /// Names, `$replace` variables, ids and editor data are not fixed up, they stay as they are in their files
    pub fn flatten(&self) -> Flattened {
        let root = &self.files[0];
        
        // not root.clone(): that copies every root brush and entity only to throw them out right after
        let mut vmf = VmfFile {
            // no path, so the flattened map can't get saved over the real root by accident
            path: None,
            versioninfo: root.versioninfo.clone(),
            visgroups: root.visgroups.clone(),
            viewsettings: root.viewsettings.clone(),
            world: World {
                key_values: root.world.key_values.clone(),
                groups: root.world.groups.clone(),
                extra: root.world.extra.clone(),
                ..Default::default()
            },
            cameras: root.cameras.clone(),
            cordons: root.cordons.clone(),
            extra_blocks: root.extra_blocks.clone(),
            ..Default::default()
        };

        let mut sources = Vec::new();
        for (index, inst) in self.instances.iter().enumerate() {
            let file = &self.files[inst.file];
            
            for solid in &file.world.solids {
                let mut solid = solid.clone();
                inst.transform.apply_solid(&mut solid);
                vmf.world.solids.push(solid);
            }

            for (idx, ent) in file.entities.iter().enumerate() {
                if is_class(ent, "func_instance") || is_class(ent, "func_instance_parms") {
                    continue;
                }
                let mut ent = ent.clone();
                inst.transform.apply_entity(&mut ent);
                vmf.entities.push(ent);
                sources.push((index, EntId { hidden: false, idx: idx as u32 }));
            }
        }

        Flattened { vmf, sources }
    }

    /// Writes EVERY FILE back to the path it was loaded from
    pub fn save(&self) -> VmfResult<()> {
        for vmf in &self.files {
            if let Some(path) = &vmf.path {
                vmf.save(path)?;
            }
        }

        Ok(())
    }

    /// Resolves an instance's `file` key and returns its index in `files`, loading it if needed
    fn load(&mut self, name: &str, dir: Option<&Path>, placer: &str) -> Option<usize> {
        if name.trim().is_empty() {
            return None;
        }

        let Some(path) = resolve(name, dir, &self.search_dirs) else {
            self.warnings
                .push(format!("{placer}: instance '{name}' not found"));
            return None;
        };
        let loaded = self
            .files
            .iter()
            .position(|vmf| vmf.path.as_deref().map(Path::new) == Some(path.as_path()));
        
            if loaded.is_some() {
            return loaded;
        }

        match VmfFile::open(&path) {
            Ok(vmf) => {
                self.files.push(vmf);
                Some(self.files.len() - 1)
            }
            Err(err) => {
                self.warnings
                    .push(format!("{placer}: instance '{}': {err}", path.display()));
                None
            }
        }
    }
}

fn is_class(ent: &Entity, class: &str) -> bool {
    ent.classname()
        .is_some_and(|name| name.eq_ignore_ascii_case(class))
}

/// Search order of `CInstancingHelper::ResolveInstancePath`: next to the VMF that places the instance,
/// then under the outermost `maps` directory above that VMF, then `search_dirs`
// todo: case-sens and shit like this - check & improve
fn resolve(name: &str, dir: Option<&Path>, search_dirs: &[PathBuf]) -> Option<PathBuf> {
    let mut relative = PathBuf::from(name.trim().replace('\\', "/"));
    relative.set_extension("vmf");
    let maps = dir.and_then(|dir| {
        dir.ancestors()
            .filter(|dir| {
                dir.file_name()
                    .is_some_and(|name| name.eq_ignore_ascii_case("maps"))
            })
            .last()
    });
    dir.into_iter()
        .chain(maps)
        .chain(search_dirs.iter().map(PathBuf::as_path))
        .map(|base| base.join(&relative))
        .find(|candidate| candidate.is_file())?
        .canonicalize()
        .ok()
}
