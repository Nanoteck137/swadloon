use std::{fs::File, path::PathBuf};

// TODO(patrik): List
//  - Add indication of full update
//  - Add better progress bar

use clap::Parser;
use serde::{Deserialize, Serialize};
use swadloon::{anilist::Metadata, ChapterEntry};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    collection_path: PathBuf,
    out_dir: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChapterMetadata {
    index: usize,
    name: String,
    pages: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MangaMetadata {
    id: String,
    title: String,
    cover: String,

    chapters: Vec<ChapterMetadata>,
}

const MANGA_CUID_LENGTH: u16 = 8;

fn download_image(name: &str, url: &str, dest: &PathBuf) -> PathBuf {
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

fn main() {
    env_logger::init();

    let constructor =
        cuid2::CuidConstructor::new().with_length(MANGA_CUID_LENGTH);

    let args = Args::parse();
    println!("Args: {:#?}", args);

    std::fs::create_dir_all(&args.out_dir).unwrap();

    for path in args.collection_path.read_dir().unwrap() {
        let path = path.unwrap();
        let path = path.path();

        println!("Path: {:#?}", path);

        let mut chapters_dir = path.clone();
        chapters_dir.push("chapters");

        let mut images_dir = path.clone();
        images_dir.push("images");

        let mut chapters_file = path.clone();
        chapters_file.push("chapters.json");

        let mut metadata_file = path.clone();
        metadata_file.push("metadata.json");

        let mut manga_file = path.clone();
        manga_file.push("manga.json");

        if !chapters_file.exists() && !metadata_file.exists() {
            println!("Invalid collection entry: {:?}", path);
            continue;
        }

        std::fs::create_dir_all(&images_dir).unwrap();

        let s = std::fs::read_to_string(chapters_file).unwrap();
        let chapters = serde_json::from_str::<Vec<ChapterEntry>>(&s).unwrap();

        let s = std::fs::read_to_string(metadata_file).unwrap();
        let metadata = serde_json::from_str::<Metadata>(&s).unwrap();

        let id = constructor.create_id();
        println!("Id: {:?}", id);

        let cover = download_image(
            "cover",
            &metadata.cover_image.extra_large,
            &images_dir,
        );

        let cover = cover.file_name().unwrap().to_str().unwrap().to_string();

        let manga = MangaMetadata {
            id: id.clone(),
            title: metadata.title.english.unwrap_or(metadata.title.romaji),
            cover,
            chapters: chapters
                .iter()
                .map(|chapter| {
                    let mut chapter_dir = chapters_dir.clone();
                    chapter_dir.push(chapter.index.to_string());

                    let mut pages = Vec::new();
                    for entry in chapter_dir.read_dir().unwrap() {
                        let entry = entry.unwrap();
                        let entry = entry.path();

                        let name = entry
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string();

                        let page_num =
                            entry.file_stem().unwrap().to_string_lossy();
                        let page_num = page_num.parse::<usize>().unwrap();
                        pages.push((page_num, name));
                    }

                    pages.sort_by(|l, r| l.0.cmp(&r.0));

                    let pages = pages
                        .iter()
                        .map(|page| page.1.clone())
                        .collect::<Vec<_>>();

                    ChapterMetadata {
                        index: chapter.index,
                        name: chapter.name.clone(),
                        pages,
                    }
                })
                .collect::<Vec<_>>(),
        };

        std::fs::write(
            manga_file,
            serde_json::to_string_pretty(&manga).unwrap(),
        )
        .unwrap();

        let mut dest = args.out_dir.clone();
        dest.push(id);
        std::os::unix::fs::symlink(path, dest).unwrap();
    }
}
