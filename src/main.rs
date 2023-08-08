use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
    process::Command,
};

use clap::Parser;
use regex::Regex;
use serde::{Deserialize, Serialize};

mod server;

#[derive(Serialize, Deserialize, Debug)]
pub struct MangaSpec {
    name: String,
    #[serde(rename = "malUrl")]
    mal_url: String,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Args {
    path: PathBuf,
    endpoint: String,
}

fn read_manga_spec<P>(manga_spec: P) -> Option<MangaSpec>
where
    P: AsRef<Path>,
{
    let s = read_to_string(manga_spec).ok()?;

    serde_json::from_str::<MangaSpec>(&s).ok()
}

#[derive(Clone, Debug)]
struct LocalChapter {
    index: usize,
    name: String,
    path: PathBuf,
}

fn get_local_chapters<P>(chapters_path: P) -> Option<Vec<LocalChapter>>
where
    P: AsRef<Path>,
{
    let paths = std::fs::read_dir(chapters_path).ok()?;

    let regex = Regex::new(r"\[(\d+)\]_\w+_([\d.]+).pdf").unwrap();

    let mut res = Vec::new();
    for path in paths {
        let path = path.ok()?;
        let path = path.path();

        let filename = path.file_name()?.to_string_lossy();
        let captures = regex.captures(&filename)?;

        let index = captures[1].parse::<usize>().ok()?;
        let name = &captures[2];

        res.push(LocalChapter {
            index,
            name: name.to_string(),
            path,
        });
    }

    res.sort_by(|l, r| l.index.cmp(&r.index));

    Some(res)
}

fn main() {
    let args = Args::parse();
    println!("Args: {:#?}", args);

    let server = server::Server::new(args.endpoint);

    let path = args.path;

    if !path.is_dir() {
        panic!("Path is not a directory");
    }

    let mut manga_spec = path.clone();
    manga_spec.push("manga.json");

    let mut cover_path = path.clone();
    cover_path.push("cover.png");

    let mut chapters_path = path.clone();
    chapters_path.push("chapters");

    let mut processed_path = path.clone();
    processed_path.push("processed");

    if !chapters_path.is_dir() {
        panic!("No chapters directory inside {:?}", path);
    }

    if !processed_path.is_dir() {
        std::fs::create_dir(&processed_path).unwrap();
    }

    let manga_spec = read_manga_spec(manga_spec).unwrap();
    let manga = if let Some(manga) = server.get_manga(&manga_spec.name) {
        manga
    } else {
        server.create_manga(&manga_spec, cover_path).unwrap()
    };

    let manga_chapters = server.get_chapters(&manga).unwrap();
    println!("Manga Chapters: {}", manga_chapters.len());
    println!(
        "{:?}",
        manga_chapters.iter().map(|i| i.index).collect::<Vec<_>>()
    );

    let local_chapters = get_local_chapters(chapters_path).unwrap();
    println!("Local: {:#?}", local_chapters);

    let mut missing_chapters = Vec::new();

    for local in local_chapters {
        let res = manga_chapters.iter().find(|i| i.index == local.index);
        if res.is_none() {
            missing_chapters.push(local);
        }
    }

    println!("Missing Chapters: {}", missing_chapters.len());

    let mut threads = Vec::new();
    for missing in missing_chapters.iter() {
        if is_already_processed(&processed_path, &missing) {
            println!("{} is already processed", missing.index);
        } else {
            let mut p = processed_path.clone();
            p.push(missing.index.to_string());
            std::fs::create_dir(&p).unwrap();

            let c = missing.clone();
            let handle = std::thread::spawn(|| {
                process_chapter(c, p);
            });
            threads.push(handle);
        }
    }

    for handle in threads {
        handle.join().unwrap();
    }

    let regex = Regex::new(r"page-(\d+).png").unwrap();

    for missing in missing_chapters {
        let mut path = processed_path.clone();
        path.push(missing.index.to_string());

        let mut pages = Vec::new();
        let items = std::fs::read_dir(path).unwrap();
        for item in items {
            let item = item.unwrap();
            let path = item.path();
            let filename = path.file_name().unwrap().to_string_lossy();
            let captures = regex.captures(&filename).unwrap();
            let page_num = captures[1].parse::<usize>().unwrap();
            pages.push((page_num, path.to_string_lossy().to_string()));
        }

        pages.sort_by(|l, r| l.0.cmp(&r.0));

        let pages = pages.into_iter().map(|i| i.1).collect::<Vec<_>>();
        server.add_chapter(&manga, missing.index, missing.name, &pages);
    }
}

fn is_already_processed<P>(processed_path: P, missing: &LocalChapter) -> bool
where
    P: AsRef<Path>,
{
    let mut path = processed_path.as_ref().to_path_buf();
    path.push(missing.index.to_string());

    path.is_dir()
}

fn process_chapter<P>(chapter: LocalChapter, path: P)
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
}
//
// fn process_manga(path: PathBuf) {
//     if !path.is_dir() {
//         panic!("Path is not a directory");
//     }
//
//     let mut manga_spec = path.clone();
//     manga_spec.push("manga.json");
//
//     let mut chapters_dir = path.clone();
//     chapters_dir.push("chapters");
//
//     let mut processed_json = path.clone();
//     processed_json.push("processed.json");
//
//     let mut processed_dir = path.clone();
//     processed_dir.push("processed");
//
//     if !processed_dir.is_dir() {
//         std::fs::create_dir(&processed_dir).unwrap();
//     }
//
//     if !chapters_dir.is_dir() {
//         panic!("No chapters directory inside path");
//     }
//
//     let manga = read_manga_spec(&manga_spec).unwrap();
//     println!("Manga Spec: {:#?}", manga);
//
//     let paths = std::fs::read_dir(chapters_dir)
//         .unwrap()
//         .map(|res| res.unwrap().path())
//         .collect::<Vec<_>>();
//
//     let regex = Regex::new(r"\[(\d+)\]_\w+_([\d.]+).pdf").unwrap();
//
//     let mut chapters = Vec::new();
//     for path in paths {
//         let file_name = path.file_name().unwrap().to_string_lossy();
//
//         let cap = regex.captures(&file_name).unwrap();
//         let index = &cap[1];
//         let ch_name = &cap[2];
//
//         chapters.push(ChapterSpec {
//             index: index.parse::<usize>().unwrap(),
//             path: path.to_path_buf(),
//             name: ch_name.to_string(),
//         });
//     }
//
//     let mut unprocessed_chapters = Vec::new();
//
//     let mut processed_chapters;
//
//     if processed_json.is_file() {
//         let s = read_to_string(&processed_json).unwrap();
//         processed_chapters = serde_json::from_str::<Vec<ProcessedChapter>>(&s).unwrap();
//
//         for ch in chapters.iter() {
//             let res = processed_chapters.iter().find(|i| i.index == ch.index);
//             if res.is_none() {
//                 unprocessed_chapters.push(ch.clone());
//             }
//         }
//     } else {
//         unprocessed_chapters = chapters;
//         processed_chapters = Vec::new();
//     }
//
//     unprocessed_chapters.sort_by(|l, r| l.index.cmp(&r.index));
//     // println!("Chapters: {:#?}", chapters);
//
//     let mut threads = Vec::new();
//     for chapter in unprocessed_chapters.iter() {
//         let mut path = processed_dir.clone();
//         path.push(chapter.index.to_string());
//         if path.is_dir() {
//             std::fs::remove_dir_all(&path).unwrap();
//             println!(
//                 "'{} - {}' exists inside processed dir",
//                 chapter.index, chapter.name
//             );
//         }
//
//         std::fs::create_dir(&path).unwrap();
//
//         let ch = chapter.clone();
//         let p = path.clone();
//         let handle = std::thread::spawn(|| {
//             return process_chapter(ch, p);
//         });
//         threads.push(handle);
//     }
//
//     for handle in threads {
//         let res = handle.join().unwrap();
//
//         processed_chapters.push(ProcessedChapter {
//             index: res.0.index,
//             name: format!("Ch. {}", res.0.name),
//             pages: res.1,
//         });
//     }
//
//     processed_chapters.sort_by(|l, r| l.index.cmp(&r.index));
//
//     let s = serde_json::to_string_pretty(&processed_chapters).unwrap();
//     let mut file = File::create(processed_json).unwrap();
//     writeln!(file, "{}", s).unwrap();
// }
