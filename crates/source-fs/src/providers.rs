//! How a game install turns into search paths.

use std::path::PathBuf;

pub use crate::gameinfo::{GameRoots, SearchPath};
use crate::gameinfo::{read_block, read_search_paths};
use crate::Error;

/// Builds the search path list for a game: what `gameinfo.txt` lists plus
/// what the game's exe mounts on its own (Portal 2 DLC folders, `mount.cfg`).
///
/// For a game or mod whose exe mounts something gameinfo does not mention,
/// combine the building blocks of this module:
///
/// ```no_run
/// use source_fs::FileSystem;
/// use source_fs::providers::{self, GameInfoProvider, GameRoots, SearchPath, SimpleGameInfo};
///
/// /// Portal 2 mod with an extra `shared_assets/` folder next to the game dirs
/// struct MyMod;
///
/// impl GameInfoProvider for MyMod {
///     fn search_paths(roots: &GameRoots) -> source_fs::Result<Vec<SearchPath>> {
///         let mut paths = providers::portal2_dlc(roots);
///         paths.push(SearchPath::new("game", roots.base_dir.join("shared_assets")));
///         paths.extend(SimpleGameInfo::search_paths(roots)?);
///         Ok(paths)
///     }
/// }
///
/// # fn main() -> source_fs::Result<()> {
/// let fs: FileSystem = FileSystem::load_from_path::<MyMod>("Portal 2/mymod".as_ref(), &Default::default())?;
/// # Ok(())
/// # }
/// ```
pub trait GameInfoProvider {
    /// Search paths in priority order, highest first. Paths that don't exist
    /// are skipped by the loader.
    fn search_paths(roots: &GameRoots) -> crate::Result<Vec<SearchPath>>;
}

/// Plain `gameinfo.txt`, nothing hardcoded
pub struct SimpleGameInfo;

impl GameInfoProvider for SimpleGameInfo {
    fn search_paths(roots: &GameRoots) -> Result<Vec<SearchPath>, Error> {
        read_search_paths(roots)
    }
}

/// `gameinfo.txt` followed by the games listed in `cfg/mount.cfg` (Garry's Mod, P2CE)
pub struct SimpleWithMount;

impl GameInfoProvider for SimpleWithMount {
    fn search_paths(roots: &GameRoots) -> Result<Vec<SearchPath>, Error> {
        let mut paths = read_search_paths(roots)?;
        paths.extend(mount_cfg(roots)?);
        Ok(paths)
    }
}

/// Portal 2: `update/` and `portal2_dlcN/` on top of `gameinfo.txt`
pub struct P2GameInfo;

impl GameInfoProvider for P2GameInfo {
    fn search_paths(roots: &GameRoots) -> Result<Vec<SearchPath>, Error> {
        let mut paths = portal2_dlc(roots);
        paths.extend(read_search_paths(roots)?);
        Ok(paths)
    }
}

/// Search paths of every game listed in `<game_dir>/cfg/mount.cfg`.
/// Entries without a `gameinfo.txt` are skipped, like the game does.
pub fn mount_cfg(roots: &GameRoots) -> Result<Vec<SearchPath>, Error> {
    let Ok(text) = std::fs::read_to_string(roots.game_dir.join("cfg").join("mount.cfg")) else {
        return Ok(Vec::new());
    };

    let mut paths = Vec::new();
    for (_, location) in read_block(&text, "mountcfg").unwrap_or_default() {
        let Ok(mounted) = GameRoots::new(&roots.game_dir.join(location), None) else {
            continue;
        };
        paths.extend(read_search_paths(&mounted)?);
    }
    Ok(paths)
}

/// The Portal 2 exe mounts `update/`, then `portal2_dlcN..1` (scan stops at the
/// first missing number), above everything from gameinfo, as both `game` and `mod`.
pub fn portal2_dlc(roots: &GameRoots) -> Vec<SearchPath> {
    let mut dlcs: Vec<PathBuf> = (1..)
        .map(|n| roots.base_dir.join(format!("portal2_dlc{n}")))
        .take_while(|dir| dir.is_dir())
        .collect();
    dlcs.reverse();

    std::iter::once(roots.base_dir.join("update"))
        .filter(|dir| dir.is_dir())
        .chain(dlcs)
        .map(|dir| SearchPath::new("game+mod", dir))
        .collect()
}
