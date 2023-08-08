use std::io::Write;
use std::{
    fs::{read_to_string, File},
    path::{Path, PathBuf},
    process::Command,
};

use clap::{Parser, Subcommand};
use regex::Regex;
use reqwest::blocking::multipart::Form;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
struct Manga {
    name: String,
    mal_url: String,
    cover: PathBuf,
}

impl Manga {
    fn to_form(&self) -> Form {
        Form::new()
            .text("name", self.name.clone())
            .text("malUrl", self.mal_url.clone())
            .file("cover", self.cover.clone())
            .unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct MangaSpec {
    name: String,
    #[serde(rename = "malUrl")]
    mal_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ChapterSpec {
    index: usize,
    path: PathBuf,
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ProcessedChapter {
    index: usize,
    name: String,
    pages: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MangaResponse {
    #[serde(rename = "collectionId")]
    collection_id: String,
    #[serde(rename = "collectionName")]
    collection_name: String,

    id: String,
    #[serde(rename = "malUrl")]
    mal_url: String,
    name: String,
    cover: String,

    created: String,
    updated: String,
}

fn create_manga(endpoint: &str, manga: &Manga) -> Option<MangaResponse> {
    let client = reqwest::blocking::Client::new();

    let form = manga.to_form();

    let collection = "manga";
    let res = client
        .post(format!(
            "{}/api/collections/{}/records",
            endpoint, collection
        ))
        .multipart(form)
        .send()
        .unwrap();

    if res.status().is_success() {
        res.json::<MangaResponse>().ok()
    } else {
        None
    }
}

fn test_create_chapter() {
    let endpoint = "http://127.0.0.1:8090";

    let manga = Manga {
        name: "Oshi no Ko".to_string(),
        mal_url: "https://myanimelist.net/manga/126146/Oshi_no_Ko".to_string(),
        cover: PathBuf::from("/home/nanoteck137/wallpaper.png"),
    };

    // let res = create_manga(&manga);
    // println!("Res: {:#?}", res);
    // return;

    let client = reqwest::blocking::Client::new();

    let form = Form::new()
        .text("num", 0.to_string())
        .text("name", "Ch. 102")
        .text("manga", "18f6sxhqycha8z9")
        .file("pages", "/home/nanoteck137/p/page-000.png")
        .unwrap()
        .file("pages", "/home/nanoteck137/p/page-001.png")
        .unwrap()
        .file("pages", "/home/nanoteck137/p/page-002.png")
        .unwrap()
        .file("pages", "/home/nanoteck137/p/page-003.png")
        .unwrap()
        .file("pages", "/home/nanoteck137/p/page-004.png")
        .unwrap()
        .file("pages", "/home/nanoteck137/p/page-005.png")
        .unwrap();

    let collection = "chapters";
    let res = client
        .post(format!(
            "{}/api/collections/{}/records",
            endpoint, collection
        ))
        .multipart(form)
        .send()
        .unwrap();
    println!("Res: {:#?}", res.json::<serde_json::Value>());
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    ProcessManga {
        path: PathBuf,
    },
    CreateManga {
        endpoint: String,
        path: PathBuf,
    },
    PushChapters {
        endpoint: String,
        path: PathBuf,
        manga_id: String,
    },
}

fn read_manga_spec<P>(manga_spec: P) -> Option<MangaSpec>
where
    P: AsRef<Path>,
{
    let s = read_to_string(manga_spec).ok()?;

    serde_json::from_str::<MangaSpec>(&s).ok()
}

fn process_chapter<P>(chapter: ChapterSpec, path: P) -> (ChapterSpec, Vec<String>)
where
    P: AsRef<Path>,
{
    // pdfimages -png \[0001\]_Chapter_1.pdf ./page
    let path = path.as_ref();
    let mut out_path = path.to_path_buf();
    out_path.push("page");

    let status = Command::new("pdfimages")
        .arg("-png")
        .arg(&chapter.path)
        .arg(out_path)
        .status()
        .unwrap();
    println!("{} - Status: {}", chapter.index, status.code().unwrap());

    let paths = std::fs::read_dir(path)
        .unwrap()
        .map(|res| res.unwrap().path())
        .collect::<Vec<_>>();
    // println!("Paths: {:?}", paths);

    let regex = Regex::new(r"page-(\d+).png").unwrap();

    let mut pages = Vec::new();
    for path in paths {
        let filename = path.file_name().unwrap().to_string_lossy();
        let captures = regex.captures(&filename).unwrap();
        let page_num = (&captures[1]).parse::<usize>().unwrap();
        pages.push((page_num, path));
    }

    pages.sort_by(|l, r| l.0.cmp(&r.0));

    let pages = pages
        .iter()
        .map(|p| {
            let file = p.1.file_name().unwrap().to_string_lossy();
            return String::from(file);
        })
        .collect::<Vec<_>>();

    return (chapter, pages);
}

fn process_manga(path: PathBuf) {
    if !path.is_dir() {
        panic!("Path is not a directory");
    }

    let mut manga_spec = path.clone();
    manga_spec.push("manga.json");

    let mut chapters_dir = path.clone();
    chapters_dir.push("chapters");

    let mut processed_json = path.clone();
    processed_json.push("processed.json");

    let mut processed_dir = path.clone();
    processed_dir.push("processed");

    if !processed_dir.is_dir() {
        std::fs::create_dir(&processed_dir).unwrap();
    }

    if !chapters_dir.is_dir() {
        panic!("No chapters directory inside path");
    }

    let manga = read_manga_spec(&manga_spec).unwrap();
    println!("Manga Spec: {:#?}", manga);

    let paths = std::fs::read_dir(chapters_dir)
        .unwrap()
        .map(|res| res.unwrap().path())
        .collect::<Vec<_>>();

    let regex = Regex::new(r"\[(\d+)\]_\w+_([\d.]+).pdf").unwrap();

    let mut chapters = Vec::new();
    for path in paths {
        let file_name = path.file_name().unwrap().to_string_lossy();

        let cap = regex.captures(&file_name).unwrap();
        let index = &cap[1];
        let ch_name = &cap[2];

        chapters.push(ChapterSpec {
            index: index.parse::<usize>().unwrap(),
            path: path.to_path_buf(),
            name: ch_name.to_string(),
        });
    }

    let mut unprocessed_chapters = Vec::new();

    let mut processed_chapters;

    if processed_json.is_file() {
        let s = read_to_string(&processed_json).unwrap();
        processed_chapters = serde_json::from_str::<Vec<ProcessedChapter>>(&s).unwrap();

        for ch in chapters.iter() {
            let res = processed_chapters.iter().find(|i| i.index == ch.index);
            if res.is_none() {
                unprocessed_chapters.push(ch.clone());
            }
        }
    } else {
        unprocessed_chapters = chapters;
        processed_chapters = Vec::new();
    }

    unprocessed_chapters.sort_by(|l, r| l.index.cmp(&r.index));
    // println!("Chapters: {:#?}", chapters);

    let mut threads = Vec::new();
    for chapter in unprocessed_chapters.iter() {
        let mut path = processed_dir.clone();
        path.push(chapter.index.to_string());
        if path.is_dir() {
            std::fs::remove_dir_all(&path).unwrap();
            println!(
                "'{} - {}' exists inside processed dir",
                chapter.index, chapter.name
            );
        }

        std::fs::create_dir(&path).unwrap();

        let ch = chapter.clone();
        let p = path.clone();
        let handle = std::thread::spawn(|| {
            return process_chapter(ch, p);
        });
        threads.push(handle);
    }

    for handle in threads {
        let res = handle.join().unwrap();

        processed_chapters.push(ProcessedChapter {
            index: res.0.index,
            name: format!("Ch. {}", res.0.name),
            pages: res.1,
        });
    }

    processed_chapters.sort_by(|l, r| l.index.cmp(&r.index));

    let s = serde_json::to_string_pretty(&processed_chapters).unwrap();
    let mut file = File::create(processed_json).unwrap();
    writeln!(file, "{}", s).unwrap();
}

fn main() {
    let args = Args::parse();
    println!("Args: {:#?}", args);

    match args.command {
        Commands::ProcessManga { path } => process_manga(path),
        Commands::CreateManga { endpoint, path } => {
            if !path.is_dir() {
                panic!("Path is not a directory");
            }

            let mut manga_spec = path.clone();
            manga_spec.push("manga.json");

            let spec = read_manga_spec(manga_spec).unwrap();

            let mut cover = path.clone();
            cover.push("cover.png");

            let manga = Manga {
                name: spec.name,
                mal_url: spec.mal_url,
                cover,
            };

            let res = create_manga(&endpoint, &manga);
            println!("Res: {:#?}", res);
        }

        Commands::PushChapters {
            endpoint,
            path,
            manga_id,
        } => {
            // let client = reqwest::blocking::Client::new();
            // let res = client
            //     .get(format!("{}/api/collections/chapters/records", endpoint))
            //     .send()
            //     .unwrap();
            //
            // if res.status().is_success() {
            //     println!("Res: {:?}", res.json::<serde_json::Value>());
            // } else {
            //     println!("Res: {:#?}", res);
            // }

            if !path.is_dir() {
                panic!("Path is not a directory");
            }

            let mut processed_json = path.clone();
            processed_json.push("processed.json");

            // let mut manga_spec = path.clone();
            // manga_spec.push("manga.json");

            let s = read_to_string(processed_json).unwrap();
            let chapters = serde_json::from_str::<Vec<ProcessedChapter>>(&s).unwrap();

            let client = reqwest::blocking::Client::new();

            for chapter in chapters {
                let mut form = Form::new()
                    .text("num", chapter.index.to_string())
                    .text("name", chapter.name)
                    .text("manga", manga_id.clone());

                for page in chapter.pages {
                    let mut path = path.clone();
                    path.push("processed");
                    path.push(chapter.index.to_string());
                    path.push(page);
                    form = form
                        .file("pages", path)
                        .unwrap();
                }

                let collection = "chapters";
                let res = client
                    .post(format!(
                        "{}/api/collections/{}/records",
                        endpoint, collection
                    ))
                    .multipart(form)
                    .send()
                    .unwrap();
                println!("Res: {:#?}", res.json::<serde_json::Value>());
            }
        }
    }
}
