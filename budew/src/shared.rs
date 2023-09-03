use std::path::PathBuf;

#[derive(Debug)]
pub struct ResolvedImages {
    pub banner: Option<PathBuf>,
    pub cover_medium: PathBuf,
    pub cover_large: PathBuf,
    pub cover_extra_large: PathBuf,
}
