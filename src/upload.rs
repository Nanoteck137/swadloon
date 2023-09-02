use std::path::PathBuf;

use log::{debug, error};
use serde::{Deserialize, Serialize};

use crate::{
    error::Error,
    process::{ChapterMetadata, MangaImages, MangaMetadata},
    server::Server,
};

// TODO(patrik): Same as manga.rs
#[derive(Serialize, Deserialize, Debug)]
struct ChapterEntry {
    index: usize,
    name: String,
    url: String,
    pages: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MetadataCoverImage {
    color: String,
    medium: String,
    large: String,
    #[serde(rename = "extraLarge")]
    extra_large: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct MetadataDate {
    day: Option<usize>,
    month: Option<usize>,
    year: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MetadataTitle {
    english: String,
    native: String,
    romaji: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Metadata {
    id: usize,
    #[serde(rename = "idMal")]
    mal_id: usize,
    title: MetadataTitle,
    status: String,

    #[serde(rename = "type")]
    typ: String,
    format: String,

    description: String,
    genres: Vec<String>,

    chapters: Option<usize>,
    volumes: Option<usize>,

    #[serde(rename = "bannerImage")]
    banner_image: String,
    #[serde(rename = "coverImage")]
    cover_image: MetadataCoverImage,

    #[serde(rename = "startDate")]
    start_date: MetadataDate,
    #[serde(rename = "endDate")]
    end_date: MetadataDate,
}

pub fn upload_single_new(path: PathBuf, server: &Server) {
    debug!("Upload '{:?}'", path);

    let mut chapter_json = path.clone();
    chapter_json.push("chapters.json");

    let mut metadata_json = path.clone();
    metadata_json.push("metadata.json");

    let mut images_dir = path.clone();
    images_dir.push("images");

    let mut chapters_dir = path.clone();
    chapters_dir.push("chapters");

    if !chapter_json.is_file() {
        panic!("No 'chapters.json' present inside '{:?}'", path);
    }

    if !metadata_json.is_file() {
        panic!("No 'metadata.json' present inside '{:?}'", path);
    }

    if !images_dir.is_dir() {
        panic!("No 'images' directory present inside '{:?}'", path);
    }

    if !chapters_dir.is_dir() {
        panic!("No 'chapters' directory present inside '{:?}'", path);
    }

    let s = std::fs::read_to_string(chapter_json).unwrap();
    let chapters = serde_json::from_str::<Vec<ChapterEntry>>(&s).unwrap();

    let s = std::fs::read_to_string(metadata_json).unwrap();
    let metadata = serde_json::from_str::<Metadata>(&s).unwrap();

    let images = images_dir
        .read_dir()
        .unwrap()
        .map(|i| i.unwrap().path())
        .collect::<Vec<_>>();

    let banner = images
        .iter()
        .filter(|i| i.file_stem().unwrap() == "banner")
        .next();
    let cover_medium = images
        .iter()
        .filter(|i| i.file_stem().unwrap() == "cover_medium")
        .next();
    let cover_large = images
        .iter()
        .filter(|i| i.file_stem().unwrap() == "cover_large")
        .next();
    let cover_extra_large = images
        .iter()
        .filter(|i| i.file_stem().unwrap() == "cover_extra_large")
        .next();

    let new_metadata = MangaMetadata {
        mal_id: metadata.mal_id,
        anilist_id: metadata.id,

        english_title: metadata.title.english.clone(),
        native_title: metadata.title.native.clone(),
        romaji_title: metadata.title.romaji.clone(),

        anilist_url: format!("https://anilist.co/manga/{}", metadata.id),
        mal_url: format!("https://myanimelist.net/manga/{}", metadata.mal_id),

        description: metadata.description.clone(),

        start_date: "2020-04-02".to_string(),
        end_date: "2020-04-02".to_string(),

        color: metadata.cover_image.color,

        images: MangaImages {
            banner: banner.unwrap().clone(),
            cover_medium: cover_medium.unwrap().clone(),
            cover_large: cover_large.unwrap().clone(),
            cover_extra_large: cover_extra_large.unwrap().clone(),
        },
    };

    let manga = match server.get_manga(metadata.mal_id) {
        Ok(manga) => {
            println!(
                "Updating manga {} '{}'",
                metadata.mal_id, metadata.title.english
            );
            let manga = server.update_manga(&manga, &new_metadata).unwrap();
            manga
        }

        Err(Error::ServerNoRecord) => {
            println!(
                "Creating new manga {} '{}'",
                metadata.mal_id, metadata.title.english
            );
            server
                .create_manga(&new_metadata)
                .expect("Failed to create new manga on the server")
        }

        Err(e) => {
            error!("Upload failed: {:?}", e);
            return;
        }
    };

    let server_chapters = server.get_chapters(&manga).unwrap();

    for chapter in chapters {
        let mut dir = chapters_dir.clone();
        dir.push(chapter.index.to_string());

        let mut pages = Vec::new();

        for page in dir.read_dir().unwrap() {
            let page = page.unwrap();
            let page = page.path();

            let page_num = page
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .parse::<usize>()
                .unwrap();

            pages.push((page_num, page));
        }

        pages.sort_by(|l, r| l.0.cmp(&r.0));

        let pages = pages.into_iter().map(|i| i.1).collect::<Vec<_>>();

        let metadata = ChapterMetadata {
            index: chapter.index,
            name: chapter.name.clone(),
            cover: pages[0].clone(),
        };

        if let Some(chapter) =
            server_chapters.iter().find(|i| i.idx == chapter.index)
        {
            println!("Updating {:4} '{}'", chapter.idx, chapter.name);
            server.update_chapter(chapter, &metadata, None).unwrap();
        } else {
            println!("Adding   {:4} '{}'", chapter.index, chapter.name);
            server.add_chapter(&manga, &metadata, &pages).unwrap();
        }
    }
}

pub fn upload_new(dir: PathBuf, endpoint: String, manga: Option<String>) {
    let server = Server::new(endpoint);
    if let Some(_manga) = manga {
        // TODO(patrik): Support
        unimplemented!("Uploading single manga in not supported right now");

        // NOTE(patrik): Just download a single entry inside the list
        // if let Some(entry) = manga_list.iter().find(|i| i.id == manga) {
        //     download_single(&dir, entry);
        // } else {
        //     error!("'{}' is not inside the manga list", manga);
        // }
    } else {
        for path in dir.read_dir().unwrap() {
            let path = path.unwrap();
            let path = path.path();

            if path.is_dir() {
                upload_single_new(path, &server);
            }
        }
    }
}
