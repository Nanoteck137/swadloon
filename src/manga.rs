use std::{
    collections::VecDeque,
    fs::File,
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::shared::{Chapters, Metadata};

#[derive(Serialize, Deserialize, Debug)]
pub struct MangaListEntry {
    pub id: String,
    pub anilist_id: String,
    pub name: String,
}

struct ThreadJob {
    referer: String,
    url: String,
    dest: PathBuf,
}

fn thread_worker(tid: usize, queue: Arc<Mutex<VecDeque<ThreadJob>>>) {
    let client = Client::new();

    'work_loop: loop {
        let mut work = {
            let mut lock = queue.lock().unwrap();
            if let Some(job) = lock.pop_front() {
                job
            } else {
                break 'work_loop;
            }
        };

        println!("{} working on '{}'", tid, work.url);

        let mut res = client
            .get(work.url)
            .header("Referer", &work.referer)
            .send()
            .unwrap();
        if !res.status().is_success() {
            // TODO(patrik): Add error queue
            panic!("Failed to download");
        }

        let content_type =
            res.headers().get("content-type").unwrap().to_str().unwrap();
        let ext = match content_type {
            "image/jpeg" => "jpeg",
            "image/png" => "png",
            _ => panic!("Unknown Content-Type '{}'", content_type),
        };

        work.dest.set_extension(ext);
        let mut file = File::create(&work.dest).unwrap();
        res.copy_to(&mut file).unwrap();
    }
}

pub fn process_single(path: PathBuf) {
    let mut chapter_json = path.clone();
    chapter_json.push("chapters.json");

    let mut metadata_json = path.clone();
    metadata_json.push("metadata.json");

    if !chapter_json.is_file() {
        panic!("No 'chapters.json' present inside '{:?}'", path);
    }

    if !metadata_json.is_file() {
        panic!("No 'metadata.json' present inside '{:?}'", path);
    }

    let mut chapter_dest = path.clone();
    chapter_dest.push("chapters");
    std::fs::create_dir_all(&chapter_dest).unwrap();

    let mut image_dest = path.clone();
    image_dest.push("images");
    std::fs::create_dir_all(&image_dest).unwrap();

    let s = std::fs::read_to_string(chapter_json).unwrap();
    let chapters = serde_json::from_str::<Chapters>(&s).unwrap();

    let s = std::fs::read_to_string(metadata_json).unwrap();
    let metadata = serde_json::from_str::<Metadata>(&s).unwrap();

    let client = Client::new();

    let images_inside = image_dest
        .read_dir()
        .unwrap()
        .map(|i| i.unwrap().path())
        .collect::<Vec<_>>();

    let process_image = |name: &str, url: &str| {
        let has_image = images_inside
            .iter()
            .filter(|i| i.file_stem().unwrap() == name)
            .next();
        if has_image.is_some() {
            println!("Skipping downloading '{}'", name);
            return;
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

        let mut filepath = image_dest.clone();
        filepath.push(name);
        filepath.set_extension(ext);

        let mut file = File::create(&filepath).unwrap();
        res.copy_to(&mut file).unwrap();
    };

    process_image("banner", &metadata.banner_image);
    process_image("cover_medium", &metadata.cover_image.medium);
    process_image("cover_large", &metadata.cover_image.large);
    process_image("cover_extra_large", &metadata.cover_image.extra_large);
}

pub fn process(dir: PathBuf, manga: Option<String>) {
    if let Some(manga) = manga {
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
                process_single(path);
            }
        }
    }
}
