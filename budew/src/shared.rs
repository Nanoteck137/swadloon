use std::path::PathBuf;

use serde::{Serialize, Deserialize};

#[derive(Debug)]
pub struct ResolvedImages {
    pub banner: PathBuf,
    pub cover_medium: PathBuf,
    pub cover_large: PathBuf,
    pub cover_extra_large: PathBuf,
}
