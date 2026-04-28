use std::path::{Path, PathBuf};

use crate::{GameInfoProvider, PackFile};

/// A dummy implementation that ignores VPK files.
/// Forces the FileSystem to read exclusively from physical disk directories.
pub struct DummyVpk;

impl PackFile for DummyVpk {
    fn open<P: AsRef<Path>>(_path: P) -> Option<Self> {
        // Always return None to indicate no VPK was loaded.
        None
    }

    fn has_entry(&self, _path: &str) -> bool {
        false
    }

    fn read_entry(&self, _path: &str) -> Option<Vec<u8>> {
        None
    }
}

/// A basic VDF parser specifically for extracting SearchPaths from gameinfo.txt
pub struct SimpleGameInfo;

impl GameInfoProvider for SimpleGameInfo {
    fn get_search_paths<P: AsRef<Path>>(path: P) -> Option<Vec<(String, String)>> {
        let content = std::fs::read_to_string(&path).ok()?;
        let path_ref = path.as_ref();
        let mut paths = Vec::new();
        let mut in_search_paths = false;


        // TODO: EXRREMELY naive parser. use a `source-kv` parser after first release
        // but it works, so it's good enough for now
        for line in content.lines() {
            let line = line.trim().to_lowercase();

            if line.contains("\"searchpaths\"") || line.contains("searchpaths") {
                in_search_paths = true;
                continue;
            }

            if in_search_paths {
                if line == "}" {
                    break;
                }

                if line == "{" || line.is_empty() || line.starts_with("//") {
                    continue;
                }

                let parts: Vec<&str> = line
                    .trim()
                    .splitn(2, char::is_whitespace)
                    .map(|s| s.trim_start().trim_matches('"'))
                    .collect();

                if parts.len() >= 2 {
                    let resolved = crate::utils::resolve_macro_path(parts[1], path_ref);
                    paths.push((parts[0].to_string(), resolved.to_string_lossy().to_string()));
                }
            }
        }

        if paths.is_empty() {
            None
        } else {
            Some(paths)
        }
    }
}

/// A GameInfoProvider that also parses `mount.cfg` for additional search paths,
/// typically found in Source engine games like Garry's Mod or P2CE.
pub struct SimpleWithMount;

impl GameInfoProvider for SimpleWithMount {
    fn get_search_paths<P: AsRef<Path>>(path: P) -> Option<Vec<(String, String)>> {
        let path_ref = path.as_ref();
        let main_game_dir = path_ref.parent()?;

        let mut res_paths = SimpleGameInfo::get_search_paths(path_ref)?;

        let mount_cfg_path = main_game_dir.join("cfg").join("mount.cfg");

        if let Ok(mount_cfg_content) = std::fs::read_to_string(&mount_cfg_path) {
            // Naive KV parser for mount.cfg
            for line in mount_cfg_content.lines() {
                let line = line.trim();
                if line.starts_with("//") || line.is_empty() || line == "{" || line == "}" || line.contains("mountcfg") {
                    continue;
                }

                let parts: Vec<&str> = line
                    .splitn(2, |c: char| c.is_whitespace())
                    .map(|s| s.trim().trim_matches('"'))
                    .filter(|s| !s.is_empty())
                    .collect();

                if parts.len() >= 2 {
                    let mount_path = parts[1];
                    let mount_game_dir = if Path::new(mount_path).is_absolute() {
                        PathBuf::from(mount_path)
                    } else {
                        main_game_dir.join(mount_path)
                    };

                    let mount_info_path = mount_game_dir.join("gameinfo.txt");
                    if mount_info_path.exists() {
                        if let Some(mounted_paths) = SimpleGameInfo::get_search_paths(&mount_info_path) {
                            res_paths.extend(mounted_paths);
                        }
                    }
                }
            }
        }

        if res_paths.is_empty() {
            None
        } else {
            Some(res_paths)
        }
    }
}


/// Portal 2 has a unique feature: DLC folders.
/// They aren't added to SearchPaths;
/// the game automatically mounts the content if it exists, incrementing the DLC number
pub struct P2GameInfo;

impl GameInfoProvider for P2GameInfo {
    fn get_search_paths<P: AsRef<Path>>(path: P) -> Option<Vec<(String, String)>> {

        // only for portal 2:
        let path_ref = path.as_ref();
        let game_path = path_ref.ancestors().nth(2).unwrap_or_else(|| Path::new(""));

        let mut paths: Vec<_> = (1..)
            .map(|idx| format!("portal2_dlc{}/", idx))
            .take_while(|dlc_name| game_path.join(dlc_name).exists())
            .map(|dlc_name| ("game".to_string(), dlc_name))
            .collect();

        paths.reverse();
        paths.extend(SimpleGameInfo::get_search_paths(&path)?);

        Some(paths)
    }
}

// TODO: Add other unique GameInfoProviders here
