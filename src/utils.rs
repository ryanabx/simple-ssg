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

pub fn check_has_index(target_path: &Path) -> bool {
    target_path.join("index.dj").exists() || target_path.join("index.djot").exists()
}

#[macro_export]
macro_rules! warn_or_error {
    // This pattern matches when there is a format string followed by arguments
    ($no_warn:expr, $fmt:expr, $($arg:tt)*) => {
        if $no_warn {
            return Err(anyhow::anyhow!($fmt, $($arg)*))
        } else {
            log::warn!($fmt, $($arg)*);
        }
    };
    // This pattern matches when there is only a format string (no arguments)
    ($no_warn:expr, $fmt:expr) => {
        if $no_warn {
            return Err(anyhow::anyhow!($fmt))
        } else {
            log::warn!($fmt);
        }
    };
}
