use serde::{Serialize, Deserialize};

pub use error::{Error, Result};

pub mod error;
pub mod anilist;

#[derive(Serialize, Deserialize, Debug)]
pub struct ChapterMetadata {
    index: usize,
    name: String,
    pages: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MangaMetadata {
    id: String,
    title: String,
    cover: String,

    chapters: Vec<ChapterMetadata>,
}
