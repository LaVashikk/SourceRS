#[cfg(feature = "material_system")]
use std::collections::HashMap;
#[cfg(feature = "material_system")]
use std::sync::Arc;
#[cfg(feature = "material_system")]
use crate::vmt::Vmt;

#[cfg(feature = "patch_resolution")]
use source_fs::{FileSystem, traits::PackFile, providers::DummyVpk, FileSystemError};

/// MaterialSystem manages material caching and resolution.
#[cfg(all(feature = "material_system", feature = "patch_resolution"))]
pub struct MaterialSystem<P: PackFile = DummyVpk> {
    fs: Option<FileSystem<P>>,
    cache: HashMap<String, Arc<Vmt>>,
    fallback: Option<Arc<Vmt>>,
    pub search_path: String,
    pub prioritize_vpks: bool,
}

#[cfg(all(feature = "material_system", not(feature = "patch_resolution")))]
pub struct MaterialSystem {
    cache: HashMap<String, Arc<Vmt>>,
    fallback: Option<Arc<Vmt>>,
    pub search_path: String,
    pub prioritize_vpks: bool,
}

#[cfg(feature = "material_system")]
macro_rules! impl_common_material_system {
    () => {
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

        /// Returns a mutable (owned) copy of a material for instantiation.
        pub fn get_material_mut(&mut self, path: &str) -> Result<Vmt, crate::Error> {
            let arc_vmt = self.get_material(path)?;
            Ok((*arc_vmt).clone())
        }
    };
}

#[cfg(all(feature = "material_system", feature = "patch_resolution"))]
impl<P: PackFile> MaterialSystem<P> {
    /// Creates a new MaterialSystem with an optional existing FileSystem.
    pub fn new(fs: Option<FileSystem<P>>) -> Self {
        Self { 
            fs, 
            cache: HashMap::new(), 
            fallback: None,
            search_path: "game".to_string(),
            prioritize_vpks: false,
        }
    }

    /// Creates a MaterialSystem by loading a FileSystem from a game directory (containing gameinfo.txt).
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<MaterialSystem<DummyVpk>, FileSystemError> {
        let fs = source_fs::create_fs(path)?;
        Ok(MaterialSystem::<DummyVpk>::new(Some(fs)))
    }

    /// Fetches a material, resolving 'patch' inheritance if a FileSystem is present.
    pub fn get_material(&mut self, path: &str) -> Result<Arc<Vmt>, crate::Error> {
        let path_lower = path.to_lowercase();
        if let Some(cached) = self.cache.get(&path_lower) {
            return Ok(Arc::clone(cached));
        }

        if let Some(fs) = &self.fs {
            if let Some(data) = fs.read_str(&path_lower, &self.search_path, self.prioritize_vpks) {
                let mut vmt = Vmt::from_str(&data)?;
                if vmt.shader == "patch" {
                    vmt = self.resolve_patch(&vmt)?;
                }
                let arc_vmt = Arc::new(vmt);
                self.cache.insert(path_lower, Arc::clone(&arc_vmt));
                return Ok(arc_vmt);
            }
        }

        if let Some(fallback) = &self.fallback {
            Ok(Arc::clone(fallback))
        } else {
            Err(crate::Error::Message(format!("Material not found: {}", path)))
        }
    }

    fn resolve_patch(&mut self, patch: &Vmt) -> Result<Vmt, crate::Error> {
        let include_path = patch.get_string("include")
            .or_else(|| patch.get_string("$include"))
            .ok_or_else(|| crate::Error::Message("Patch material missing 'include' key".into()))?;

        let base_arc = self.get_material(&include_path)?;
        let mut base_owned = (*base_arc).clone();
        base_owned.apply_patch(patch);
        Ok(base_owned)
    }

    impl_common_material_system!();
}

#[cfg(all(feature = "material_system", not(feature = "patch_resolution")))]
impl MaterialSystem {
    /// Creates a new standalone MaterialSystem without FileSystem support.
    pub fn new() -> Self {
        Self { 
            cache: HashMap::new(), 
            fallback: None,
            search_path: "game".to_string(),
            prioritize_vpks: false,
        }
    }

    /// Fetches a material from the cache or returns the fallback.
    pub fn get_material(&mut self, path: &str) -> Result<Arc<Vmt>, crate::Error> {
        let path_lower = path.to_lowercase();
        if let Some(cached) = self.cache.get(&path_lower) {
            return Ok(Arc::clone(cached));
        }

        if let Some(fallback) = &self.fallback {
            Ok(Arc::clone(fallback))
        } else {
            Err(crate::Error::Message(format!("Material not found (MaterialSystem is standalone): {}", path)))
        }
    }

    impl_common_material_system!();
}
