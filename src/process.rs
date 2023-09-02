use std::{
    fs::File,
    path::{Path, PathBuf},
    process::Command,
};

use log::{debug, error, info};
use regex::Regex;
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::manga::{read_manga_list, MangaListEntry};

#[derive(Serialize, Deserialize, Debug)]
pub struct MangaImages {
    pub banner: PathBuf,
    pub cover_medium: PathBuf,
    pub cover_large: PathBuf,
    pub cover_extra_large: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MangaMetadata {
    #[serde(rename = "malId")]
    pub mal_id: usize,
    #[serde(rename = "anilistId")]
    pub anilist_id: usize,

    pub english_title: String,
    pub native_title: String,
    pub romaji_title: String,

    pub anilist_url: String,
    pub mal_url: String,

    pub description: String,

    pub start_date: String,
    pub end_date: String,

    pub color: String,

    pub images: MangaImages,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChapterMetadata {
    pub index: usize,
    pub name: String,
    pub cover: PathBuf,
}
