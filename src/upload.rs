use std::path::PathBuf;

use log::{debug, error, warn, info};

use crate::{
    error::Error,
    manga::{read_manga_list, MangaListEntry},
    process::{MangaMetadata, ChapterMetadata},
    server::{Server, Manga},
};

fn upload_chapters(manga: &Manga, server: &Server, dir: PathBuf) {
    let paths = dir.read_dir().unwrap();

    let server_chapters = server.get_chapters(manga).unwrap();

    for path in paths {
        let path = path.unwrap();
        let path = path.path();

        let mut metadata_path = path.clone();
        metadata_path.push("info.json");

        if !metadata_path.is_file() {
            warn!("{:?} has no 'info.json'", metadata_path);
            continue;
        }

        let s = std::fs::read_to_string(metadata_path).unwrap();
        let metadata = serde_json::from_str::<ChapterMetadata>(&s).unwrap();

        let pages = metadata.pages.iter().map(|i| {
            let mut p = path.clone();
            p.push(i);
            p
        }).collect::<Vec<_>>();

        if let Some(chapter) = server_chapters.iter().find(|i| i.idx == metadata.index) {
            info!("Updating chapter {}", metadata.index);
            server.update_chapter(chapter, &metadata, &pages).unwrap();
        } else {
            info!("Uploading chapter {} to server", metadata.index);
            server.add_chapter(manga, &metadata, &pages).unwrap();
        }

    }
}

pub fn upload_single(dir: &PathBuf, server: &Server, entry: &MangaListEntry) {
    debug!("Trying to upload '{}'", entry.id);

    let mut entry_dir = dir.clone();
    entry_dir.push(&entry.name);

    let mut out_dir = entry_dir.clone();
    out_dir.push("processed");

    let mut metadata_file = out_dir.clone();
    metadata_file.push("manga.json");

    let mut chapter_dir = out_dir.clone();
    chapter_dir.push("chapters");

    if !metadata_file.is_file() {
        // TODO(patrik): Better error message
        panic!("No 'manga.json' inside processed dir");
    }

    if !chapter_dir.is_dir() {
        panic!("No 'chapters' directory inside processed");
    }

    let s = std::fs::read_to_string(metadata_file).unwrap();
    let metadata = serde_json::from_str::<MangaMetadata>(&s).unwrap();

    println!("Metadata: {:#?}", metadata);

    let manga = match server.get_manga(&metadata.name) {
        Ok(manga) => {
            let manga =
                server.update_manga(&manga, out_dir, &metadata).unwrap();
            Ok(manga)
        }

        Err(Error::NoMangasWithName(_)) => {
            Ok(server.create_manga(out_dir, &metadata).unwrap())
        }

        Err(e) => Err(Error::FailedToRetriveManga(Box::new(e))),
    }
    .unwrap();

    println!("Manga: {:#?}", manga);

    upload_chapters(&manga, server, chapter_dir);
}

pub fn upload(dir: PathBuf, endpoint: String, manga: Option<String>) {
    let manga_list = read_manga_list(&dir);

    let server = Server::new(endpoint);

    if let Some(manga) = manga {
        // NOTE(patrik): Process single manga
        if let Some(entry) = manga_list.iter().find(|i| i.id == manga) {
            upload_single(&dir, &server, entry);
        } else {
            error!("No manga inside list: {}", manga);
        }
    } else {
        // NOTE(patrik): Upload all the mangas inside the list
        for manga in manga_list {
            upload_single(&dir, &server, &manga);
        }
    }
}
