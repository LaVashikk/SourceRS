use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::mount::{open_loose, open_packed, read_all};
use crate::{Error, FileLocation, FileReader, Mount, PackFile, Result, utils};

/// Virtual filesystem: search path IDs mapped to their mounts in priority order.
#[derive(Debug)]
pub struct FileSystem<P: PackFile = crate::DefaultPack> {
    root_path: PathBuf,
    search_paths: HashMap<String, Vec<Mount<P>>>,
}

impl<P: PackFile> Clone for FileSystem<P> {
    fn clone(&self) -> Self {
        Self {
            root_path: self.root_path.clone(),
            search_paths: self.search_paths.clone()
        }
    }
}

/// Borrowed result of a lookup, so `open` can hand out readers tied to `&self`
enum Hit<'a, P> {
    Loose(PathBuf),
    Packed(&'a Arc<P>),
}

impl<P: PackFile> FileSystem<P> {
    /// Empty filesystem. Relative paths passed to `mount_*` resolve against `root_path`.
    pub fn new(root_path: impl Into<PathBuf>) -> Self {
        Self {
            root_path: root_path.into(),
            search_paths: HashMap::new(),
        }
    }

    /// Appends a loose directory to `search_path` (lowest priority so far)
    pub fn mount_dir(&mut self, search_path: &str, dir: impl AsRef<Path>) {
        let dir = self.root_path.join(dir);
        self.push_mount(search_path, Mount::Dir(dir));
    }

    /// Appends a pack to `search_path` (lowest priority so far)
    pub fn mount_vpk(&mut self, search_path: &str, vpk_path: impl AsRef<Path>) -> Result<()> {
        let path = self.root_path.join(vpk_path);
        if let Some(pack) = P::open(&path).map_err(|e| Error::pack(path, e))? {
            self.push_mount(search_path, Mount::Pack(Arc::new(pack)));
        }
        Ok(())
    }

    /// The same location reached twice keeps only its first (highest) position
    pub(crate) fn push_mount(&mut self, search_path: &str, mount: Mount<P>) {
        let mounts = self.search_paths.entry(search_path.to_lowercase()).or_default();
        if !mounts.contains(&mount) {
            mounts.push(mount);
        }
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    pub fn search_paths(&self) -> &HashMap<String, Vec<Mount<P>>> {
        &self.search_paths
    }

    /// Mounts of one search path ID, highest priority first
    pub fn mounts(&self, search_path: &str) -> &[Mount<P>] {
        // todo: remove alloc here
        self.search_paths
            .get(&search_path.to_lowercase())
            .map_or(&[], Vec::as_slice)
    }

    fn locate(&self, file_path: &str, search_path: &str) -> Option<(Hit<'_, P>, String)> {
        let normalized = utils::normalize_path(file_path);
        let hit = self.mounts(search_path).iter().find_map(|mount| match mount {
            Mount::Dir(dir) => utils::resolve_path_case_insensitive(dir, &normalized).map(Hit::Loose),
            Mount::Pack(pack) => pack.has_entry(&normalized).then_some(Hit::Packed(pack)),
        })?;

        Some((hit, normalized))
    }

    /// Finds which mount serves a virtual path without opening the file.
    ///
    /// Unlike [`find_asset`](Self::find_asset), expects an exact, fully-qualified
    /// relative path (e.g. `"materials/models/player/body.vmt"`).
    pub fn find(&self, file_path: &str, search_path: &str) -> Option<FileLocation<P>> {
        let (hit, normalized) = self.locate(file_path, search_path)?;
        Some(match hit {
            Hit::Loose(path) => FileLocation::Loose(path),
            Hit::Packed(pack) => FileLocation::Packed {
                pack: Arc::clone(pack),
                entry: normalized,
            },
        })
    }

    pub fn exists(&self, path: &str, search_path: &str) -> bool {
        self.locate(path, search_path).is_some()
    }

    /// Opens a file from the mounted paths without reading it into memory.
    pub fn open(&self, file_path: &str, search_path: &str) -> Result<FileReader<'_, P>> {
        let Some((hit, normalized)) = self.locate(file_path, search_path) else {
            return Err(Error::not_found(file_path, search_path));
        };
        match hit {
            Hit::Loose(path) => open_loose(&path),
            Hit::Packed(pack) => open_packed(pack.as_ref(), &normalized),
        }
    }

    pub fn read(&self, file_path: &str, search_path: &str) -> Result<Vec<u8>> {
        self.open(file_path, search_path)
            .and_then(|file| read_all(file, file_path))
    }

    pub fn read_str(&self, file_path: &str, search_path: &str) -> Result<String> {
        self.read(file_path, search_path)
            .and_then(|data| into_string(data, file_path))
    }

    pub fn read_material(&self, name: &str, search_path: &str) -> Result<String> {
        let path = asset_path(name, "materials", "vmt");
        self.read_str(&path, search_path)
    }

    pub fn read_model(&self, name: &str, search_path: &str) -> Result<Vec<u8>> {
        let path = asset_path(name, "models", "mdl");
        self.read(&path, search_path)
    }

    pub fn read_sound(&self, name: &str, search_path: &str) -> Result<Vec<u8>> {
        let path = asset_path(name, "sound", "");
        self.read(&path, search_path)
    }

    /// Finds an asset by name, folder, and extension without opening the file.
    ///
    /// Wraps the name using [`asset_path`] (e.g. `find_asset("models/player/body", "materials", "vmt", "game")`
    /// resolves to `"materials/models/player/body.vmt"`).
    pub fn find_asset(&self, name: &str, folder: &str, ext: &str, search_path: &str) -> Option<FileLocation<P>> {
        self.find(&asset_path(name, folder, ext), search_path)
    }
}

/// Turns an asset reference as written in VMT/MDL/VMF (`metal/wall`, `materials/metal/wall.vmt`)
/// into its VFS path, adding `folder/` and `.ext` only when missing
pub fn asset_path<'a>(name: &'a str, folder: &str, ext: &str) -> Cow<'a, str> {
    let folder = folder.trim_matches(['/', '\\']);
    let ext = ext.trim_start_matches('.');
    let clean_name = name.trim_matches(['/', '\\']);

    let has_folder = has_prefix_folder(clean_name, folder);
    let has_ext = has_suffix_ext(clean_name, ext);

    if has_folder && has_ext {
        // hooray - happy path
        return Cow::Borrowed(clean_name);
    }

    let mut path = String::with_capacity(
        clean_name.len() + folder.len() + ext.len() + 2
    );

    if !has_folder {
        path.push_str(folder);
        path.push('/');
    }

    path.push_str(clean_name);

    if !has_ext {
        path.push('.');
        path.push_str(ext);
    }

    Cow::Owned(path)
}

fn into_string(data: Vec<u8>, path: &str) -> Result<String> {
    String::from_utf8(data).map_err(|source| Error::Utf8 {
        path: path.to_owned(),
        source,
    })
}

fn has_prefix_folder(name: &str, folder: &str) -> bool {
    if folder.is_empty() {
        return true;
    }
    match name.get(..folder.len()) {
        Some(prefix) if prefix.eq_ignore_ascii_case(folder) => {
            name[folder.len()..].starts_with(['/', '\\'])
        }
        _ => false,
    }
}

fn has_suffix_ext(name: &str, ext: &str) -> bool {
    if ext.is_empty() {
        return true;
    }
    match name.rsplit_once('.') {
        Some((stem, suffix)) => !stem.is_empty() && suffix.eq_ignore_ascii_case(ext),
        None => false,
    }
}
