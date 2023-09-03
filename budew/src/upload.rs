use std::{fs::File, path::PathBuf};

use log::{debug, error};
use reqwest::blocking::Client;
use swadloon::{anilist::Metadata, Chapters};

use crate::{error::Error, server::Server, shared::ResolvedImages};

pub fn upload_single(path: PathBuf, server: &Server) {
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

    if !chapters_dir.is_dir() {
        panic!("No 'chapters' directory present inside '{:?}'", path);
    }

    if !images_dir.is_dir() {
        std::fs::create_dir_all(&images_dir).unwrap();
    }

    let s = std::fs::read_to_string(chapter_json).unwrap();
    let chapters = serde_json::from_str::<Chapters>(&s).unwrap();

    let s = std::fs::read_to_string(metadata_json).unwrap();
    let metadata = serde_json::from_str::<Metadata>(&s).unwrap();

    let images = images_dir
        .read_dir()
        .unwrap()
        .map(|i| i.unwrap().path())
        .collect::<Vec<_>>();

    let client = Client::new();

    let process_image = |name: &str, url: &str| -> PathBuf {
        let has_image = images
            .iter()
            .filter(|i| i.file_stem().unwrap() == name)
            .next();

        if let Some(path) = has_image {
            println!("Skipping downloading '{}'", name);
            return path.clone();
        }

        let mut res = client.get(url).send().unwrap();

        let content_type =
            res.headers().get("content-type").unwrap().to_str().unwrap();
        println!("Content Type: {:?}", content_type);

        let ext = match content_type {
            "image/jpeg" => "jpeg",
            "image/png" => "png",
            _ => unimplemented!("Unknown content type: {}", content_type),
        };

        println!("Ext: {}", ext);

        let mut filepath = images_dir.clone();
        filepath.push(name);
        filepath.set_extension(ext);

        let mut file = File::create(&filepath).unwrap();
        res.copy_to(&mut file).unwrap();

        filepath
    };

    let banner = if let Some(banner_iamge) = &metadata.banner_image {
        Some(process_image("banner", banner_iamge))
    } else {
        None
    };
    let cover_medium =
        process_image("cover_medium", &metadata.cover_image.medium);
    let cover_large =
        process_image("cover_large", &metadata.cover_image.large);
    let cover_extra_large =
        process_image("cover_extra_large", &metadata.cover_image.extra_large);

    let images = ResolvedImages {
        banner,
        cover_medium,
        cover_large,
        cover_extra_large,
    };

    let manga = match server.get_manga(metadata.mal_id.unwrap()) {
        Ok(manga) => {
            println!(
                "Updating manga {} '{}'",
                metadata.mal_id.unwrap(),
                metadata.title.english.as_ref().unwrap_or(&metadata.title.romaji)
            );
            let manga =
                server.update_manga(&manga, &metadata, &images).unwrap();
            manga
        }

        Err(Error::ServerNoRecord) => {
            println!(
                "Creating new manga {} '{}'",
                metadata.mal_id.unwrap(),
                metadata
                    .title
                    .english
                    .as_ref()
                    .unwrap_or(&metadata.title.romaji),
            );
            server
                .create_manga(&metadata, &images)
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

        let cover = pages[0].clone();

        if let Some(server_chapter) =
            server_chapters.iter().find(|i| i.idx == chapter.index)
        {
            println!("Updating {:4} '{}'", chapter.index, chapter.name);
            server
                .update_chapter(server_chapter, &chapter, cover, None)
                .unwrap();
        } else {
            println!("Adding   {:4} '{}'", chapter.index, chapter.name);
            server.add_chapter(&manga, &chapter, cover, &pages).unwrap();
        }
    }
}

pub fn upload(dir: PathBuf, endpoint: String, manga: Option<String>) {
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
                upload_single(path, &server);
            }
        }
    }
}
