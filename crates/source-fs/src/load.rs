use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::gameinfo::is_vpk;
use crate::providers::{GameInfoProvider, GameRoots, SearchPath};
use crate::{Error, FileSystem, Mount, PackFile};

/// Options for [`FileSystem`] initialization
#[derive(Debug, Clone, Default)]
pub struct FileSystemOptions {
    /// Subdirectory under `bin/` for platform binaries (e.g. `"win64"`, `"linux64"`)
    pub bin_platform: Option<String>,
    /// Engine root to resolve gameinfo paths against.
    /// Needed for sourcemods, whose `hl2`/`platform` live in the SDK Base install.
    /// Defaults to the parent of the game dir
    pub base_dir: Option<PathBuf>,
}

impl<P: PackFile> FileSystem<P> {
    /// Locates the game using the local Steam installation and loads the filesystem.
    #[cfg(feature = "steam")]
    pub fn load_from_app_id<G: GameInfoProvider>(
        app_id: u32,
        game_name: &str,
        options: &FileSystemOptions,
    ) -> Result<Self, Error> {
        let steamdir = steamlocate::SteamDir::locate().map_err(|_| Error::SteamNotFound)?;
        let (app, library) = steamdir
            .find_app(app_id)?
            .ok_or(Error::SteamAppNotFound(app_id))?;
        let game_path = library.resolve_app_dir(&app).join(game_name);

        Self::load_from_path::<G>(&game_path, options)
    }

    /// Loads the filesystem from a specific game directory (where `gameinfo.txt` resides).
    pub fn load_from_path<G: GameInfoProvider>(
        game_path: &Path,
        options: &FileSystemOptions,
    ) -> Result<Self, Error> {
        let roots = GameRoots::new(game_path, options.base_dir.as_deref())?;
        let search_paths = G::search_paths(&roots)?;

        let mut fs = Self::new(&roots.base_dir);
        let mut loader = Loader {
            fs: &mut fs,
            packs: HashMap::new(),
        };
        for search_path in &search_paths {
            loader.mount(search_path)?;
        }

        fs.mount_engine_defaults(&roots, options);
        Ok(fs)
    }

    /// Paths the engine adds on its own, matching `path` output of retail Portal 2
    fn mount_engine_defaults(&mut self, roots: &GameRoots, options: &FileSystemOptions) {
        if let Some(platform) = &options.bin_platform {
            let dir = self.root_path().join("bin").join(platform);
            if dir.is_dir() {
                self.mount_dir("executable_path", dir);
            }
        }
        self.mount_dir("executable_path", "bin");
        self.mount_dir("executable_path", "");

        let game_dir = roots.game_dir.clone();
        for (id, dir) in [
            ("platform", PathBuf::from("platform")),
            ("default_write_path", game_dir.clone()),
            ("logdir", game_dir),
            ("config", PathBuf::from("platform/config")),
        ] {
            if self.mounts(id).is_empty() {
                self.mount_dir(id, dir);
            }
        }

        if !self.mounts("game").is_empty() {
            self.mount_dir("game", "platform");
        }
    }
}

struct Loader<'a, P: PackFile> {
    fs: &'a mut FileSystem<P>,
    /// Archives reachable through several gameinfo lines are opened once
    packs: HashMap<PathBuf, Arc<P>>,
}

impl<P: PackFile> Loader<'_, P> {
    fn mount(&mut self, search_path: &SearchPath) -> Result<(), Error> {
        self.mount_location(&search_path.ids, &search_path.path)
    }

    fn mount_location(&mut self, ids: &[String], path: &Path) -> Result<(), Error> {
        if path.file_name().is_some_and(|name| name == "*") {
            let parent = path.parent().unwrap_or(path);
            return self.mount_wildcard(ids, parent);
        }
        if is_vpk(path) {
            return self.mount_pack(ids, path);
        }
        if path.is_dir() {
            return self.mount_dir(ids, path);
        }
        Ok(())
    }

    /// `dir/*`: every subdirectory and VPK inside, alphabetically
    fn mount_wildcard(&mut self, ids: &[String], dir: &Path) -> Result<(), Error> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Ok(());
        };

        let mut children: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir() || (is_vpk(path) && !is_vpk_chunk(path)))
            .collect();
        children.sort_by_cached_key(|path| path.to_string_lossy().to_lowercase());

        for child in children {
            self.mount_location(ids, &child)?;
        }
        Ok(())
    }

    fn mount_pack(&mut self, ids: &[String], path: &Path) -> Result<(), Error> {
        // gameinfo names multi-chunk archives without the `_dir` suffix
        let path = if path.exists() {
            path.to_path_buf()
        } else {
            let Some(stem) = path.file_stem() else { return Ok(()) };
            let dir_vpk = path.with_file_name(format!("{}_dir.vpk", stem.to_string_lossy()));
            if !dir_vpk.exists() {
                return Ok(());
            }
            dir_vpk
        };

        if let Some(pack) = self.open_pack(&path)? {
            for id in ids {
                self.fs.push_mount(id, Mount::Pack(Arc::clone(&pack)));
            }
        }
        Ok(())
    }

    /// A directory brings its own `*_dir.vpk` archives, mounted above its loose files
    fn mount_dir(&mut self, ids: &[String], dir: &Path) -> Result<(), Error> {
        let mut archives: Vec<PathBuf> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.to_ascii_lowercase().ends_with("_dir.vpk"))
            })
            .collect();
        archives.sort();

        for archive in archives {
            if let Some(pack) = self.open_pack(&archive)? {
                for id in ids {
                    self.fs.push_mount(id, Mount::Pack(Arc::clone(&pack)));
                }
            }
        }
        for id in ids {
            self.fs.push_mount(id, Mount::Dir(dir.to_path_buf()));
        }
        Ok(())
    }

    fn open_pack(&mut self, path: &Path) -> Result<Option<Arc<P>>, Error> {
        if let Some(pack) = self.packs.get(path) {
            return Ok(Some(Arc::clone(pack)));
        }
        let Some(pack) = P::open(path).map_err(|error| Error::pack(path.to_path_buf(), error))? else {
            return Ok(None);
        };

        let pack = Arc::new(pack);
        self.packs.insert(path.to_path_buf(), Arc::clone(&pack));
        Ok(Some(pack))
    }
}

/// `foo_000.vpk` data chunks belong to `foo_dir.vpk` and are never mounted on their own
fn is_vpk_chunk(path: &Path) -> bool {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| stem.rsplit_once('_'))
        .is_some_and(|(_, suffix)| suffix.len() == 3 && suffix.bytes().all(|b| b.is_ascii_digit()))
}
