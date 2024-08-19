use std::path::{Path, PathBuf};

pub fn get_relative_path(full_path: &Path, base_path: &Path) -> Option<PathBuf> {
    // Ensure the full path starts with the base path
    if full_path.starts_with(base_path) {
        // Strip the base path to get the relative path
        let relative_path = full_path.strip_prefix(base_path).ok()?;
        Some(relative_path.to_path_buf())
    } else {
        None
    }
}
