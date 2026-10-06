use std::path::{Path, PathBuf};

/// Lowercase, forward slashes, no leading slash: the form VPK entries use.
pub(crate) fn normalize_path(path: &str) -> String {
    let path = path.replace('\\', "/").to_ascii_lowercase();
    match path.strip_prefix('/') {
        Some(stripped) => stripped.to_owned(),
        None => path,
    }
}

/// Resolves a file path case-insensitively. Native fast path for Windows.
#[cfg(windows)]
pub(crate) fn resolve_path_case_insensitive(base_dir: &Path, relative_path: &str) -> Option<PathBuf> {
    let full_path = base_dir.join(relative_path);
    full_path.is_file().then_some(full_path)
}

/// Resolves a file path case-insensitively by iterating through directory contents.
/// Required for Unix file systems where asset casing might not match the request.
#[cfg(unix)]
pub(crate) fn resolve_path_case_insensitive(base_dir: &Path, relative_path: &str) -> Option<PathBuf> {
    // One stat covers the common case where casing already matches
    let exact = base_dir.join(relative_path);
    if exact.is_file() {
        return Some(exact);
    }

    // todo: cache directory listings T_T
    let mut current_path = base_dir.to_path_buf();
    let mut components = Path::new(relative_path)
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .peekable();

    while let Some(name) = components.next() {
        let entry = std::fs::read_dir(&current_path)
            .ok()?
            .flatten()
            .find(|entry| entry.file_name().to_string_lossy().eq_ignore_ascii_case(name))?;
        current_path = entry.path();

        if components.peek().is_some() && !current_path.is_dir() {
            return None;
        }
    }

    current_path.is_file().then_some(current_path)
}
