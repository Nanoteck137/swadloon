use serde::{Serialize, Deserialize};

pub use error::{Error, Result};

pub mod error;
pub mod anilist;

#[derive(Serialize, Deserialize, Debug)]
pub struct ChapterMetadata {
    pub index: usize,
    pub name: String,
    pub pages: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MangaMetadata {
    pub id: String,
    pub title: String,
    pub cover: String,

    pub chapters: Vec<ChapterMetadata>,
}
