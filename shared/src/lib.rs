use serde::{Serialize, Deserialize};

pub mod anilist;

#[derive(Serialize, Deserialize, Debug)]
pub struct ChapterEntry {
    pub index: usize,
    pub name: String,
    pub page_count: usize,
}

pub type Chapters = Vec<ChapterEntry>;

