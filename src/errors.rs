use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SsgError {
    #[error("index.{{dj|djot}} not found! consider creating one in the base target directory as the default page.")]
    IndexPageNotFound,
    #[error("Path {0} is not relative to target directory")]
    PathNotRelative(PathBuf),
    #[error("An entry returned error {0}")]
    DirEntryError(walkdir::Error),
    #[error("Referenced file path {0} does not exist!")]
    LinkError(PathBuf),
}
