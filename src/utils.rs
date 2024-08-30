use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};

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

pub fn get_template_if_exists(
    djot_document_path: &Path,
    root_path: &Path,
) -> anyhow::Result<Option<String>> {
    if !is_ancestor(root_path, djot_document_path) {
        Err(anyhow::anyhow!("Root path is not an ancestor of main path"))
    } else {
        let mut current = PathBuf::from(djot_document_path.parent().unwrap());
        loop {
            let template_file = current.join("template.html");
            log::trace!("Checking for template file at {:?}", &template_file);
            if template_file.exists() {
                return Ok(Some(read_to_string(&template_file)?));
            }
            if current == root_path {
                break;
            }
            current = current.parent().unwrap().to_path_buf();
        }
        Ok(None)
    }
}

/// Checks if `ancestor` is an ancestor of `descendant`.
fn is_ancestor(ancestor: &Path, descendant: &Path) -> bool {
    let mut current = PathBuf::from(descendant);
    while let Some(parent) = current.parent() {
        if parent == ancestor {
            return true;
        }
        current = parent.to_path_buf();
    }
    false
}

pub fn wrap_html_content(content: &str, template: Option<&str>) -> String {
    match template {
        Some(tmpl) => tmpl.to_string().replace("<!-- {CONTENT} -->", content),
        None => content.to_string(),
    }
}
