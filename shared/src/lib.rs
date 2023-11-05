use std::{path::PathBuf, fs::File};

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

    pub description: String,

    #[serde(rename = "anilistId")]
    pub anilist_id: usize,
    #[serde(rename = "malId")]
    pub mal_id: usize,

    pub status: String,

    #[serde(rename = "startDate")]
    pub start_date: String,
    #[serde(rename = "endDate")]
    pub end_date: String,

    pub chapters: Vec<ChapterMetadata>,
}

pub const MANGA_CUID_LENGTH: u16 = 8;

pub fn gen_manga_id() -> String {
    // TODO(patrik): Lazy static?
    let constructor =
        cuid2::CuidConstructor::new().with_length(MANGA_CUID_LENGTH);
    constructor.create_id()
}

pub fn download_image(name: &str, url: &str, dest: &PathBuf) -> PathBuf {
    let dest_files = dest
        .read_dir()
        .unwrap()
        .map(|i| i.unwrap().path())
        .collect::<Vec<_>>();
    let has_image = dest_files
        .iter()
        .filter(|i| i.file_stem().unwrap() == name)
        .next();

    if let Some(path) = has_image {
        println!("Skipping downloading '{}'", name);
        return path.clone();
    }

    let client = reqwest::blocking::Client::new();
    let mut res = client.get(url).send().unwrap();

    let content_type =
        res.headers().get("content-type").unwrap().to_str().unwrap();

    let ext = match content_type {
        "image/jpeg" => "jpeg",
        "image/png" => "png",
        _ => unimplemented!("Unknown content type: {}", content_type),
    };

    let mut filepath = dest.clone();
    filepath.push(name);
    filepath.set_extension(ext);

    let mut file = File::create(&filepath).unwrap();
    res.copy_to(&mut file).unwrap();

    filepath
}
