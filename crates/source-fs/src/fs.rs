use std::{collections::HashMap, path::{Path, PathBuf}, sync::Arc};

use crate::{FileSystemError, GameInfoProvider, PackFile, utils};

#[derive(Debug, Clone, Default)]
pub struct FileSystemOptions {
    pub bin_platform: Option<String>,
}

/// Core FileSystem representation holding physical directories and loaded pack files.
#[derive(Debug)]
pub struct FileSystem<P: PackFile> {
    root_path: PathBuf,
    search_path_dirs: HashMap<String, Vec<PathBuf>>,
    search_path_vpks: HashMap<String, Vec<Arc<P>>>,
}

impl<P: PackFile> Clone for FileSystem<P> {
    fn clone(&self) -> Self {
        Self {
            root_path: self.root_path.clone(),
            search_path_dirs: self.search_path_dirs.clone(),
            search_path_vpks: self.search_path_vpks.clone(),
        }
    }
}

impl<P: PackFile> FileSystem<P> {
    /// Locates the game using the local Steam installation and loads the filesystem.
    #[cfg(feature = "steam")]
    pub fn load_from_app_id<G: GameInfoProvider>(
        app_id: u32,
        game_name: &str,
        options: &FileSystemOptions,
    ) -> Result<Self, FileSystemError> {
        let steamdir = steamlocate::SteamDir::locate().map_err(|_| FileSystemError::SteamNotFound)?;
        let (app, library) = steamdir
            .find_app(app_id)?
            .ok_or(FileSystemError::SteamAppNotFound(app_id))?;
        let game_path = library.resolve_app_dir(&app).join(&game_name);

        Self::load_from_path::<G>(&game_path, options)
    }

    /// Loads the filesystem from a specific game directory (where `gameinfo.txt` resides).
    pub fn load_from_path<G: GameInfoProvider>(
        game_path: &Path,
        options: &FileSystemOptions,
    ) -> Result<Self, FileSystemError> {
        let gameinfo_path = game_path.join("gameinfo.txt");
        if !gameinfo_path.is_file() {
            return Err(FileSystemError::GameInfoNotFound(gameinfo_path));
        }

        let root_path = game_path.parent()
            .ok_or_else(|| FileSystemError::InvalidGamePath(game_path.to_path_buf()))?
            .to_path_buf();
        let game_id = game_path.file_name()
            .ok_or_else(|| FileSystemError::InvalidGamePath(game_path.to_path_buf()))?
            .to_string_lossy()
            .to_string();

        let mut fs = Self {
            root_path,
            search_path_dirs: HashMap::new(),
            search_path_vpks: HashMap::new(),
        };

        let search_paths = G::get_search_paths(&gameinfo_path)
            .ok_or(FileSystemError::GameInfoParseError)?;
        if search_paths.is_empty() {
            return Ok(fs);
        }

        for (i, (key, value)) in search_paths.into_iter().enumerate() {
            let searches: Vec<String> = key.to_lowercase()
                .split('+')
                .map(|s| s.to_string())
                .collect();

            let mut path = value;// .to_lowercase(); // todo: case insensitive!

            if path.ends_with('.') && !path.ends_with("..") {
                path.pop();
            }
            let path = utils::normalize_slashes(&path, false, false);

            if path.ends_with(".vpk") {
                let mut full_path = fs.root_path.join(&path);

                if !full_path.exists() {
                    // Try to fallback to the `_dir.vpk` naming convention
                    if let Some(stem) = full_path.file_stem() {
                        let parent = full_path.parent().unwrap_or_else(|| Path::new(""));
                        let dir_vpk = parent.join(format!("{}_dir.vpk", stem.to_string_lossy()));
                        if dir_vpk.exists() {
                            full_path = dir_vpk;
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

                if let Some(pack) = P::open(&full_path).map(Arc::new) {
                    for search in &searches {
                        fs.search_path_vpks
                            .entry(search.clone())
                            .or_default()
                            .push(Arc::clone(&pack));
                    }
                }
            } else {
                for search in &searches {
                    if path.ends_with("/*") {
                        let glob_parent_path = fs.root_path.join(&path[..path.len() - 2]);
                        if glob_parent_path.is_dir() {
                            if let Ok(entries) = std::fs::read_dir(&glob_parent_path) {
                                for entry in entries.flatten() {
                                    let glob_child_path = utils::normalize_slashes(
                                        &entry.path().to_string_lossy(),
                                        false,
                                        false,
                                    );
                                    fs.search_path_dirs
                                        .entry(search.clone())
                                        .or_default()
                                        .push(PathBuf::from(glob_child_path));
                                }
                            }
                        }
                    } else {
                        let test_path = fs.root_path.join(&path);
                        // dbg!(&fs.root_path, &path, &test_path); // todo: debug
                        // dbg!(&test_path);
                        if test_path.exists() {
                            fs.search_path_dirs
                                .entry(search.clone())
                                .or_default()
                                .push(PathBuf::from(&path));

                            // Automatically populate `gamebin` and `mod` depending on context
                            if search == "game" {
                                fs.search_path_dirs
                                    .entry("gamebin".to_string())
                                    .or_default()
                                    .push(PathBuf::from(format!("{}/bin", path)));

                                if i == 0 {
                                    fs.search_path_dirs
                                        .entry("mod".to_string())
                                        .or_default()
                                        .push(PathBuf::from(&path));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Setup default path overrides
        let exec_paths = fs.search_path_dirs.entry("executable_path".to_string()).or_default();
        if let Some(plat) = &options.bin_platform {
            let plat_path = fs.root_path.join("bin").join(plat);
            if plat_path.exists() {
                exec_paths.push(PathBuf::from(format!("bin/{}", plat)));
            }
        }
        exec_paths.push(PathBuf::from("bin"));
        exec_paths.push(PathBuf::from(""));

        fs.search_path_dirs
            .entry("platform".to_string())
            .or_insert_with(|| vec![PathBuf::from("platform")]);

        if let Some(game_paths) = fs.search_path_dirs.get_mut("game") {
            let platform_buf = PathBuf::from("platform");
            if !game_paths.contains(&platform_buf) {
                game_paths.push(platform_buf);
            }
        }

        fs.search_path_dirs
            .entry("default_write_path".to_string())
            .or_insert_with(|| vec![PathBuf::from(&game_id)]);

        fs.search_path_dirs
            .entry("logdir".to_string())
            .or_insert_with(|| vec![PathBuf::from(&game_id)]);

        fs.search_path_dirs
            .entry("config".to_string())
            .or_insert_with(|| vec![PathBuf::from("platform/config")]);

        Ok(fs)
    }

    pub fn root_path(&self) -> &PathBuf {
        &self.root_path
    }

    pub fn search_path_dirs(&self) -> &HashMap<String, Vec<PathBuf>> {
        &self.search_path_dirs
    }

    pub fn search_path_dirs_mut(&mut self) -> &mut HashMap<String, Vec<PathBuf>> {
        &mut self.search_path_dirs
    }

    pub fn search_path_vpks(&self) -> &HashMap<String, Vec<Arc<P>>> {
        &self.search_path_vpks
    }

    pub fn search_path_vpks_mut(&mut self) -> &mut HashMap<String, Vec<Arc<P>>> {
        &mut self.search_path_vpks
    }

    /// Formats an asset path, safely preventing duplicated prefixes or suffixes.
    fn format_asset_path(name: &str, prefix: &str, suffix: &str) -> String {
        let mut path = String::with_capacity(name.len() + prefix.len() + suffix.len());
        if !prefix.is_empty() && !name.starts_with(prefix) {
            path.push_str(prefix);
        }
        path.push_str(name);
        if !suffix.is_empty() && !name.ends_with(suffix) {
            path.push_str(suffix);
        }

        path
    }

    pub fn find_file(&self, file_path: &str, search_path: &str) -> Option<PathBuf> {
        let file_path_str = utils::normalize_slashes(&file_path.to_lowercase(), true, false);
        let search_path_str = search_path.to_lowercase();

        if let Some(dirs) = self.search_path_dirs.get(&search_path_str) {
            for base_path in dirs {
                let base_dir = self.root_path.join(base_path);
                if let Some(resolved_path) = utils::resolve_path_case_insensitive(&base_dir, &file_path_str) {
                    return Some(resolved_path);
                }
            }
        }

        None
    }

    /// Reads data from the internal mounted paths using standard Source Engine priorities.
    pub fn read(&self, file_path: &str, search_path: &str, prioritize_vpks: bool) -> Option<Vec<u8>> {
        let file_path_str = utils::normalize_slashes(&file_path.to_lowercase(), true, false);
        let search_path_str = search_path.to_lowercase();

        if prioritize_vpks {
            if let Some(data) = self.check_vpks(&search_path_str, &file_path_str) {
                return Some(data);
            }
        }

        if let Some(resolved_path) = self.find_file(&file_path_str, &search_path_str) {
            if let Ok(data) = std::fs::read(resolved_path) {
                return Some(data);
            }
        }

        if !prioritize_vpks {
            return self.check_vpks(&search_path_str, &file_path_str);
        }

        None
    }

    /// Same as `read`, but takes an optional active map pack file which gets highest priority.
    pub fn read_for_map(
        &self,
        map_pack: Option<&P>,
        file_path: &str,
        search_path: &str,
        prioritize_vpks: bool,
    ) -> Option<Vec<u8>> {
        if let Some(map) = map_pack {
            if map.has_entry(file_path) {
                return map.read_entry(file_path);
            }
        }
        self.read(file_path, search_path, prioritize_vpks)
    }

    pub fn read_str(&self, file_path: &str, search_path: &str, prioritize_vpks: bool) -> Option<String> {
        let data = self.read(file_path, search_path, prioritize_vpks)?;
        // Some(String::from_utf8_lossy(&data).to_string())
        String::from_utf8(data).ok()
    }

    /// Finds an asset's PathBuf without reading its contents into memory.
    pub fn find_asset(&self, name: &str, prefix: &str, suffix: &str, search_path: &str) -> Option<PathBuf> {
        let path = Self::format_asset_path(name, prefix, suffix);
        self.find_file(&path, search_path)
    }

    /// Reads any asset as raw bytes, safely appending prefix and suffix if missing.
    pub fn read_asset(&self, name: &str, prefix: &str, suffix: &str, search_path: &str, prioritize_vpks: bool) -> Option<Vec<u8>> {
        let path = Self::format_asset_path(name, prefix, suffix);
        self.read(&path, search_path, prioritize_vpks)
    }

    /// Reads any asset as a UTF-8 string, safely appending prefix and suffix if missing.
    pub fn read_asset_str(&self, name: &str, prefix: &str, suffix: &str, search_path: &str, prioritize_vpks: bool) -> Option<String> {
        self.read_asset(name, prefix, suffix, search_path, prioritize_vpks)
            .and_then(|data| String::from_utf8(data).ok())
    }

    /// Reads a material file (.vmt).
    pub fn read_material(&self, name: &str, search_path: &str, prioritize_vpks: bool) -> Option<Vec<u8>> {
        self.read_asset(name, "materials/", ".vmt", search_path, prioritize_vpks)
    }

    /// Reads a material file (.vmt) as a UTF-8 string.
    pub fn read_material_str(&self, name: &str, search_path: &str, prioritize_vpks: bool) -> Option<String> {
        self.read_asset_str(name, "materials/", ".vmt", search_path, prioritize_vpks)
    }

    /// Reads a model file (.mdl).
    pub fn read_model(&self, name: &str, search_path: &str, prioritize_vpks: bool) -> Option<Vec<u8>> {
        self.read_asset(name, "models/", ".mdl", search_path, prioritize_vpks)
    }

    /// Reads a model file (.mdl) as a UTF-8 string.
    pub fn read_model_str(&self, name: &str, search_path: &str, prioritize_vpks: bool) -> Option<String> {
        self.read_asset_str(name, "models/", ".mdl", search_path, prioritize_vpks)
    }

    /// Reads a sound file (.wav or fallback to .mp3) as a UTF-8 string.
    pub fn read_sound_str(&self, name: &str, search_path: &str, prioritize_vpks: bool) -> Option<String> {
        // Try exact name or append .wav
        if let Some(data) = self.read_asset_str(name, "sound/", ".wav", search_path, prioritize_vpks) {
            return Some(data);
        }

        // Fallback to .mp3
        let clean_name = name.strip_suffix(".wav").unwrap_or(name);
        self.read_asset_str(clean_name, "sound/", ".mp3", search_path, prioritize_vpks)
    }

    fn find_in_vpks(&self, search_path: &str, file_path: &str) -> Option<PathBuf> {
        if let Some(vpks) = self.search_path_vpks.get(search_path) {
            for vpk in vpks {
                if vpk.has_entry(file_path) {
                    return Some(PathBuf::from(file_path));
                }
            }
        }
        None
    }

    fn check_vpks(&self, search_path: &str, file_path: &str) -> Option<Vec<u8>> {
        if let Some(vpks) = self.search_path_vpks.get(search_path) {
            for vpk in vpks {
                if vpk.has_entry(file_path) {
                    return vpk.read_entry(file_path);
                }
            }
        }
        None
    }
}
