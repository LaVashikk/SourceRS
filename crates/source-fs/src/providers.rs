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
                    paths.push((parts[0].to_string(), parts[1].to_string()));
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

/// Portal 2 has a unique feature: DLC folders.
/// They aren't added to SearchPaths;
/// the game automatically mounts the content if it exists, incrementing the DLC number
pub struct P2GameInfo;

impl GameInfoProvider for P2GameInfo {
    fn get_search_paths<P: AsRef<Path>>(path: P) -> Option<Vec<(String, String)>> {
        let mut paths = vec![];

        // only for portal 2:
        let game_path_str = path.as_ref()
            .ancestors()
            .nth(2)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let game_path = PathBuf::from(&game_path_str);

        for idx in 1..99 {
            let dlc_name = format!("portal2_dlc{}/", idx);
            let dlc_path = game_path.join(&dlc_name);
            if !dlc_path.exists() {
                break
            }
            // paths.push(("game".to_string(), dlc_path.to_string_lossy().into_owned()));
            paths.push(("game".to_string(), dlc_name));
        }

        paths.reverse();
        paths.extend(SimpleGameInfo::get_search_paths(&path)?);

        Some(paths)
    }
}

// TODO: Add other unique GameInfoProviders here
