use std::path::PathBuf;

use serde::{Serialize, Deserialize};

pub use error::{Error, Result};

pub mod error;
pub mod anilist;
pub mod server;

#[derive(Serialize, Deserialize, Debug)]
pub struct ChapterEntry {
    pub index: usize,
    pub name: String,
    pub page_count: usize,
}

pub type Chapters = Vec<ChapterEntry>;

#[derive(Debug)]
pub struct ResolvedImages {
    pub banner: Option<PathBuf>,
    pub cover_medium: PathBuf,
    pub cover_large: PathBuf,
    pub cover_extra_large: PathBuf,
}
