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
