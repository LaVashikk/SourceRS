use std::path::{Path, PathBuf};

/// Normalizes slash types ensuring paths correspond to standard internal structures.
pub(crate) fn normalize_slashes(path: &str, strip_prefix: bool, strip_suffix: bool) -> String {
    let mut p = path.replace('\\', "/");
    if strip_prefix && p.starts_with('/') {
        p.remove(0);
    }
    if strip_suffix && p.ends_with('/') {
        p.pop();
    }
    p
}

/// Resolves a file path case-insensitively. Native fast path for Windows.
#[cfg(windows)]
pub(crate) fn resolve_path_case_insensitive(base_dir: &Path, relative_path: &str) -> Option<PathBuf> {
    let full_path = base_dir.join(relative_path);
    if full_path.exists() {
        Some(full_path)
    } else {
        None
    }
}

/// Resolves Source Engine macros like |all_source_engine_paths| and |gameinfo_path|
/// and returns an absolute path.
pub fn resolve_macro_path(value: &str, gameinfo_path: &Path) -> PathBuf {
    let gameinfo_path = gameinfo_path.canonicalize().unwrap_or_else(|_| gameinfo_path.to_path_buf());
    let game_dir = gameinfo_path.parent().unwrap_or_else(|| Path::new("."));
    let engine_root = game_dir.parent().unwrap_or_else(|| Path::new("."));

    const ALL_SOURCE_ENGINE_PATHS: &str = "|all_source_engine_paths|";
    const GAMEINFO_PATH_MACRO: &str = "|gameinfo_path|";

    let resolved = if value.starts_with(ALL_SOURCE_ENGINE_PATHS) {
        engine_root.join(&value[ALL_SOURCE_ENGINE_PATHS.len()..])
    } else if value.starts_with(GAMEINFO_PATH_MACRO) {
        game_dir.join(&value[GAMEINFO_PATH_MACRO.len()..])
    } else {
        // If it's already absolute, join will just return it.
        // Otherwise, standard Source behavior is relative to engine_root (the folder containing game folders)
        engine_root.join(value)
    };

    resolved
}

/// Resolves a file path case-insensitively by iterating through directory contents.
/// Required for Unix file systems where asset casing might not match the request.
#[cfg(unix)]
pub(crate) fn resolve_path_case_insensitive(base_dir: &Path, relative_path: &str) -> Option<PathBuf> {
    use std::path::Component;
    // todo: cache it later

    let mut current_path = base_dir.to_path_buf();
    let relative_path = Path::new(relative_path);

    for component in relative_path.components() {
        let component_os_str = match component {
            Component::Normal(name) => name,
            _ => continue,
        };

        let target_name_lower = match component_os_str.to_str() {
            Some(s) => s.to_lowercase(),
            None => return None,
        };

        let mut found_match = false;

        if let Ok(entries) = std::fs::read_dir(&current_path) {
            for entry in entries.flatten() {
                let entry_name_str_lower = entry.file_name().to_string_lossy().to_lowercase();

                if entry_name_str_lower == target_name_lower {
                    current_path = entry.path();
                    found_match = true;
                    break;
                }
            }
        }

        if !found_match {
            return None;
        }

        let is_last_component = component == relative_path.components().last().unwrap();
        if !is_last_component && !current_path.is_dir() {
            return None;
        }
    }

    Some(current_path)
}
