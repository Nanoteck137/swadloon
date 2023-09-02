use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ChapterEntry {
    pub index: usize,
    pub name: String,
    pub url: String,
    pub pages: Vec<String>,
}

pub type Chapters = Vec<ChapterEntry>;

#[derive(Serialize, Deserialize, Debug)]
pub struct MetadataCoverImage {
    pub color: String,
    pub medium: String,
    pub large: String,
    #[serde(rename = "extraLarge")]
    pub extra_large: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MetadataDate {
    pub day: Option<usize>,
    pub month: Option<usize>,
    pub year: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MetadataTitle {
    pub english: String,
    pub native: String,
    pub romaji: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Metadata {
    pub id: usize,
    #[serde(rename = "idMal")]
    pub mal_id: usize,
    pub title: MetadataTitle,
    pub status: String,

    #[serde(rename = "type")]
    pub typ: String,
    pub format: String,

    pub description: String,
    pub genres: Vec<String>,

    pub chapters: Option<usize>,
    pub volumes: Option<usize>,

    #[serde(rename = "bannerImage")]
    pub banner_image: String,
    #[serde(rename = "coverImage")]
    pub cover_image: MetadataCoverImage,

    #[serde(rename = "startDate")]
    pub start_date: MetadataDate,
    #[serde(rename = "endDate")]
    pub end_date: MetadataDate,
}
