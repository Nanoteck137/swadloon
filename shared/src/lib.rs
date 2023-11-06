use std::{path::{PathBuf, Path}, fs::File};

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


pub fn read_manga_metadata<P>(manga_dir: P) -> MangaMetadata
where
    P: AsRef<Path>,
{
    let mut path = manga_dir.as_ref().to_path_buf();
    path.push("manga.json");

    let s = std::fs::read_to_string(path).unwrap();
    serde_json::from_str::<MangaMetadata>(&s).unwrap()
}

pub fn write_manga_metadata<P>(manga_dir: P, metadata: &MangaMetadata)
where
    P: AsRef<Path>,
{
    let mut path = manga_dir.as_ref().to_path_buf();
    path.push("manga.json");

    let s = serde_json::to_string_pretty(metadata).unwrap();
    std::fs::write(path, s).unwrap();
}

pub fn read_anilist_meta<P>(manga_dir: P) -> anilist::Metadata
where
    P: AsRef<Path>,
{
    let mut path = manga_dir.as_ref().to_path_buf();
    path.push("metadata.json");

    let s = std::fs::read_to_string(path).unwrap();
    serde_json::from_str::<anilist::Metadata>(&s).unwrap()
}

pub fn manga_image_dir<P>(manga_dir: P) -> PathBuf
where
    P: AsRef<Path>,
{
    let mut path = manga_dir.as_ref().to_path_buf();
    path.push("images");

    if !path.exists() {
        std::fs::create_dir(&path).unwrap();
    }

    path
}

pub fn metadata_from_anilist<P>(
    manga_dir: P,
    metadata: anilist::Metadata,
    id: String,
) -> MangaMetadata
where
    P: AsRef<Path>,
{
    let image_dir = manga_image_dir(manga_dir);
    let cover =
        download_image("cover", &metadata.cover_image.extra_large, &image_dir);
    let cover = cover.file_name().unwrap().to_str().unwrap().to_string();

    MangaMetadata {
        id,
        title: metadata.title.english.unwrap_or(metadata.title.romaji),
        cover,

        description: metadata.description,

        anilist_id: metadata.id,
        mal_id: metadata.mal_id.unwrap(),

        status: metadata.status,

        start_date: metadata.start_date.to_iso8601(),
        end_date: metadata.end_date.to_iso8601(),

        chapters: Vec::new(),
    }
}

pub fn get_manga_id<P>(manga_dir: P) -> String
where
    P: AsRef<Path>,
{
    let mut path = manga_dir.as_ref().to_path_buf();
    path.push("manga.json");

    println!("Path: {:?}", path);
    let s = std::fs::read_to_string(path).unwrap();
    let value = serde_json::from_str::<serde_json::Value>(&s).unwrap();

    if let Some(id) = value.get("id") {
        id.as_str().unwrap().to_string()
    } else {
        gen_manga_id()
    }
}

pub fn get_sorted_pages<P>(chapter_dir: P) -> Vec<String>
where
    P: AsRef<Path>,
{
    let chapter_dir = chapter_dir.as_ref();
    let mut pages = chapter_dir
        .read_dir()
        .unwrap()
        .map(|e| e.unwrap().path())
        .map(|e| e.file_name().unwrap().to_str().unwrap().to_string())
        .filter(|e| e.as_str() != "name.txt")
        .map(|e| {
            let (page_num, _) = e.split_once(".").unwrap();
            (page_num.parse::<usize>().unwrap(), e)
        })
        .collect::<Vec<_>>();
    pages.sort_by(|l, r| l.0.cmp(&r.0));

    pages.into_iter().map(|e| e.1).collect::<Vec<_>>()
}

pub fn get_chapter_name<P>(chapter_dir: P) -> String
where
    P: AsRef<Path>,
{
    let mut path = chapter_dir.as_ref().to_path_buf();
    path.push("name.txt");

    std::fs::read_to_string(path).unwrap()
}

pub fn get_chapter_index<P>(chapter_dir: P) -> usize
where
    P: AsRef<Path>,
{
    let chapter_dir = chapter_dir.as_ref();
    chapter_dir
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<usize>()
        .unwrap()
}

pub fn get_manga_chapters<P>(manga_dir: P, metadata: &mut MangaMetadata)
where
    P: AsRef<Path>,
{
    let mut chapters_dir = manga_dir.as_ref().to_path_buf();
    chapters_dir.push("chapters");

    for entry in chapters_dir.read_dir().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let index = get_chapter_index(&path);
        let name = get_chapter_name(&path);
        let pages = get_sorted_pages(&path);

        metadata
            .chapters
            .push(ChapterMetadata { index, name, pages });
    }

    metadata.chapters.sort_by(|l, r| l.index.cmp(&r.index));
}
