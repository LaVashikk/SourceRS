use std::collections::HashMap;
use std::sync::Arc;
use std::path::PathBuf;
use crate::vmt::Vmt;

use source_fs::{FileSystem, traits::PackFile, providers::DummyVpk, FileSystemError};

/// MaterialSystem manages material caching, resolution, and file path tracking.
pub struct MaterialSystem<P: PackFile = DummyVpk> {
    /// The underlying file system used to load materials.
    pub fs: FileSystem<P>,
    /// Cache of loaded materials (unresolved).
    cache: HashMap<String, Arc<Vmt>>,
    /// Cache of resolved materials (with patch inheritance applied).
    resolved_cache: HashMap<String, Arc<Vmt>>,
    /// Map of material names to their file paths.
    paths: HashMap<String, PathBuf>,
    /// Fallback material used when a requested material is not found.
    fallback: Option<Arc<Vmt>>,
    /// The search path group (e.g., "game", "mod").
    pub search_path: String,
    /// Whether to prioritize VPK files over loose files.
    pub prioritize_vpks: bool,
}

impl<P: PackFile> MaterialSystem<P> {
    /// Creates a new MaterialSystem with the given FileSystem.
    pub fn new(fs: FileSystem<P>) -> Self {
        Self {
            fs,
            cache: HashMap::new(),
            resolved_cache: HashMap::new(),
            paths: HashMap::new(),
            fallback: None,
            search_path: "game".to_string(),
            prioritize_vpks: false,
        }
    }

    /// Creates a MaterialSystem by loading a FileSystem from a game directory (containing gameinfo.txt).
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<MaterialSystem<DummyVpk>, FileSystemError> {
        let fs = source_fs::create_fs(path)?;
        Ok(MaterialSystem::<DummyVpk>::new(fs))
    }

    /// Sets a fallback material to return when a material is not found.
    pub fn set_fallback(&mut self, vmt: Vmt) {
        self.fallback = Some(Arc::new(vmt));
    }

    /// Registers a virtual/procedural material in the cache.
    pub fn register(&mut self, path: &str, vmt: Vmt) {
        self.cache.insert(path.to_lowercase(), Arc::new(vmt));
    }

    /// Sets the search path group (e.g., "game", "mod").
    pub fn with_search_path(mut self, path: &str) -> Self {
        self.search_path = path.to_string();
        self
    }

    /// Sets whether VPKs should be prioritized over loose files.
    pub fn prioritize_vpks(mut self, prioritize: bool) -> Self {
        self.prioritize_vpks = prioritize;
        self
    }

    /// Returns a material as-is (without resolving patches).
    pub fn get_material(&mut self, path: &str) -> Result<Arc<Vmt>, crate::Error> {
        let path_lower = path.to_lowercase();

        if let Some(cached) = self.cache.get(&path_lower) {
            return Ok(Arc::clone(cached));
        }

        // Try to find the file path first
        if let Some(file_path) = self.fs.find_asset(&path_lower, "materials/", ".vmt", &self.search_path) {
            self.paths.insert(path_lower.clone(), file_path);
        }

        if let Some(data) = self.fs.read_material_str(&path_lower, &self.search_path, self.prioritize_vpks) {
            let vmt = Vmt::from_str(&data)?;
            let arc_vmt = Arc::new(vmt);
            self.cache.insert(path_lower, Arc::clone(&arc_vmt));
            return Ok(arc_vmt);
        }

        if let Some(fallback) = &self.fallback {
            Ok(Arc::clone(fallback))
        } else {
            Err(crate::Error::Message(format!("Material not found: {}", path)))
        }
    }

    /// Returns a material with patch inheritance resolved.
    pub fn get_resolved_material(&mut self, path: &str) -> Result<Arc<Vmt>, crate::Error> {
        let path_lower = path.to_lowercase();

        if let Some(cached) = self.resolved_cache.get(&path_lower) {
            return Ok(Arc::clone(cached));
        }

        let vmt = self.get_material(&path_lower)?;

        if vmt.shader == "patch" {
            let resolved = self.resolve_patch(&vmt)?;
            let arc_resolved = Arc::new(resolved);
            self.resolved_cache.insert(path_lower, Arc::clone(&arc_resolved));
            return Ok(arc_resolved);
        }

        Ok(vmt)
    }

    /// Returns a mutable reference to a material in the cache.
    /// Uses get_material to ensure the material is loaded.
    /// Changes will be reflected globally.
    pub fn get_material_mut(&mut self, path: &str) -> Result<&mut Vmt, crate::Error> {
        let path_lower = path.to_lowercase();
        self.get_material(&path_lower)?;

        // Invalidate resolved cache so it gets re-resolved with new changes
        self.resolved_cache.remove(&path_lower);

        let arc = self.cache.get_mut(&path_lower)
            .ok_or_else(|| crate::Error::Message("Material missing from cache".into()))?;

        Ok(Arc::make_mut(arc))
    }

    /// Resolves 'patch' shader inheritance.
    pub fn resolve_patch(&mut self, patch: &Vmt) -> Result<Vmt, crate::Error> {
        let include_path = patch.get_string("include")
            .or_else(|| patch.get_string("$include"))
            .ok_or_else(|| crate::Error::Message("Patch material missing 'include' key".into()))?;

        // We use get_resolved_material recursively to handle nested patches
        let base_arc = self.get_resolved_material(&include_path)?;
        let mut base_owned = (*base_arc).clone();
        base_owned.apply_patch(patch);
        Ok(base_owned)
    }

    /// Returns the file path of a loaded material, if available.
    pub fn get_material_path(&self, path: &str) -> Option<&PathBuf> {
        self.paths.get(&path.to_lowercase())
    }
}
