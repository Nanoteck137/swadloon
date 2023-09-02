use serde::{Serialize, Deserialize};

pub mod anilist;

#[derive(Serialize, Deserialize, Debug)]
pub struct ChapterEntry {
    pub index: usize,
    pub name: String,
    pub url: String,
    pub pages: Vec<String>,
}

pub type Chapters = Vec<ChapterEntry>;

