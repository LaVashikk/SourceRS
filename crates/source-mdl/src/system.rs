use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use source_fs::{DefaultPack, FileSystem, PackFile, asset_path};
use source_vmt::{MaterialSystem, Vmt};

use crate::studio::VTX_EXTENSIONS;
use crate::{Error, MaterialReference, Result, StudioModel};

/// Loads and caches studio models through a game's search paths, and resolves their materials through the [`MaterialSystem`] it owns
pub struct ModelSystem<P: PackFile = DefaultPack> {
    /// Also holds the filesystem and the search path both systems use
    pub materials: MaterialSystem<P>,
    cache: HashMap<String, Arc<StudioModel>>,
}

impl ModelSystem<DefaultPack> {
    /// Loads the filesystem of a game directory (the one containing `gameinfo.txt`)
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self::new(source_fs::create_fs(path)?))
    }
}

impl<P: PackFile> ModelSystem<P> {
    pub fn new(fs: FileSystem<P>) -> Self {
        Self { materials: MaterialSystem::new(fs), cache: HashMap::new() }
    }

    /// Sets the search path group (e.g. "game", "mod") for models and materials
    pub fn with_search_path(mut self, path: &str) -> Self {
        self.materials.search_path = path.to_owned();
        self
    }

    pub fn fs(&self) -> &FileSystem<P> {
        &self.materials.fs
    }

    /// Loads a model by name (`props/box`, `models/props/box.mdl`) with the `.vvd` and `.vtx` next to it.
    /// Loaded models are cached by path
    pub fn get_model(&mut self, name: &str) -> Result<Arc<StudioModel>> {
        let path = asset_path(name, "models", "mdl")
            .replace('\\', "/")
            .to_ascii_lowercase();
        if let Some(model) = self.cache.get(&path) {
            return Ok(Arc::clone(model));
        }

        let fs = &self.materials.fs;
        let search_path = &self.materials.search_path;
        let stem = path.strip_suffix(".mdl").unwrap_or(&path);
        let vtx = VTX_EXTENSIONS
            .iter()
            .map(|extension| format!("{stem}.{extension}"))
            .find(|vtx| fs.exists(vtx, search_path))
            .unwrap_or_else(|| format!("{stem}.{}", VTX_EXTENSIONS[0]));
        let model = Arc::new(StudioModel::from_bytes(
            &fs.read(&path, search_path)?,
            &fs.read(&format!("{stem}.vvd"), search_path)?,
            &fs.read(&vtx, search_path)?,
        )?);

        self.cache.insert(path, Arc::clone(&model));
        Ok(model)
    }

    /// Resolves the VMT of a mesh material, with patch materials applied
    pub fn get_material(&mut self, reference: &MaterialReference) -> Result<Arc<Vmt>> {
        // todo todo todo here!!!!!
        let candidates = &reference.path_candidates;
        // Same as the engine: the first `$cdmaterials` dir that actually has the file wins
        // So a broken VMT there is an error, we don't quietly skip to the next dir
        // Nothing found at all? MaterialSystem reports the miss or hands out its fallback
        let path = match candidates.as_slice() {
            [only] => only.as_str(),
            _ => candidates
                .iter()
                .find(|path| {
                    self.fs()
                        .find_asset(path, "materials", "vmt", &self.materials.search_path)
                        .is_some()
                })
                .or(candidates.first())
                .map_or("", String::as_str),
        };

        self.materials
            .get_resolved_material(path)
            .map_err(|source| Error::Material { path: path.to_owned(), source })
    }
}
