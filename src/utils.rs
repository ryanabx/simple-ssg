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

pub fn warn_or_error(error: crate::errors::SsgError, no_warn: bool) -> anyhow::Result<()> {
    if no_warn {
        Err(error.into())
    } else {
        log::warn!("{}", error);
        Ok(())
    }
}

pub fn wrap_html_content(content: &str, style: Option<&str>) -> anyhow::Result<String> {
    let html_content = format!(
        "<!DOCTYPE html> \
        <html lang=\"en\"> \
        <head> \
        <meta charset=\"UTF-8\"> \
        <style> \
        {} \
        </style> \
        </head> \
        <body>
        {} \
        </body> \
        </html>
        ",
        &style.unwrap_or(""),
        content,
    );
    Ok(html_content)
}
