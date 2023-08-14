use std::{
    collections::VecDeque,
    fs::read_to_string,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};

use clap::Parser;
use log::{debug, info, trace, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use server::{Manga, Server};

use crate::error::Error;

mod error;
mod server;

#[derive(Serialize, Deserialize, Debug)]
struct MangaSpec {
    name: Option<String>,
    #[serde(rename = "malUrl")]
    mal_url: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RawMangaInfo {
    name: String,
    description_formatted: String,
    description_text: String,
    status: String,
    year: usize,
    publisher: String,
    total_issues: usize,
    publication_run: String
}

#[derive(Debug)]
pub struct MangaInfo {
    name: String,
    mal_url: String,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The pocketbase endpoint (example: http://localhost:8090)
    endpoint: String,
    /// Path to the manga directory (i.e. where the 'manga.json' is)
    path: PathBuf,

    #[arg(short, long, default_value_t = false)]
    update: bool,

    /// Number of threads for processing chapters
    #[arg(short, long, default_value_t = 1)]
    num_threads: usize,
}

fn read_manga_spec(paths: &Paths) -> Option<MangaSpec> {
    let s = read_to_string(&paths.manga_spec).ok()?;
    serde_json::from_str::<MangaSpec>(&s).ok()
}

fn read_manga_info(paths: &Paths) -> Option<MangaInfo> {
    let spec = read_manga_spec(paths)?;

    let s = read_to_string(&paths.manga_info).ok()?;

    let v = serde_json::from_str::<serde_json::Value>(&s).unwrap();
    let v = &v["metadata"];
    let v = serde_json::from_value::<RawMangaInfo>(v.clone()).unwrap();
    println!("{:#?}", v);

    Some(MangaInfo {
        name: spec.name.unwrap_or(v.name),
        mal_url: spec.mal_url,
    })
}

#[derive(Clone, Debug)]
struct LocalChapter {
    index: usize,
    name: String,
    path: PathBuf,

    pages: Vec<PathBuf>,
}

fn get_chapter_pages<P>(path: P) -> Option<Vec<PathBuf>>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let paths = path.read_dir().unwrap();

    let mut res = Vec::new();
    for path in paths {
        let path = path.ok()?;
        let path = path.path();

        let filename = path.file_stem()?.to_string_lossy();
        let page_num = filename.parse::<usize>().ok()?;
        res.push((page_num, path));
    }

    res.sort_by(|l, r| l.0.cmp(&r.0));

    Some(res.into_iter().map(|i| i.1).collect::<Vec<_>>())
}

fn get_local_chapters<P>(path: P) -> Option<Vec<LocalChapter>>
where
    P: AsRef<Path>,
{
    let paths = std::fs::read_dir(path).ok()?;

    let regex = Regex::new(r"\[(\d+)\]_\w+_([\d.]+)").ok()?;

    let mut res = Vec::new();
    for path in paths {
        let path = path.ok()?;
        let path = path.path();

        let filename = path.file_name()?.to_string_lossy();
        if let Some(captures) = regex.captures(&filename) {
            let index = captures[1].parse::<usize>().ok()?;
            let name = &captures[2];

            let pages = get_chapter_pages(&path)?;

            res.push(LocalChapter {
                index,
                name: name.to_string(),
                path,
                pages,
            });
        }
    }

    res.sort_by(|l, r| l.index.cmp(&r.index));

    Some(res)
}

fn worker_thread(
    tid: usize,
    work_queue: Arc<Mutex<VecDeque<LocalChapter>>>,
    server: Server,
    manga: Manga,
) {
    'work_loop: loop {
        let work = {
            let mut lock =
                work_queue.lock().expect("Failed to aquire work queue lock");
            if let Some(work) = lock.pop_front() {
                work
            } else {
                break 'work_loop;
            }
        };

        trace!("{}: working on: {}", tid, work.index);

        let _ = server.add_chapter(
            &manga,
            work.index,
            work.name.clone(),
            &work.pages,
        );
    }
}

struct Paths {
    base: PathBuf,
    manga_spec: PathBuf,
    manga_info: PathBuf,
    cover_path: PathBuf,
}

impl Paths {
    fn new(base: PathBuf) -> Self {
        if !base.is_dir() {
            // TODO(patrik): Better error message
            panic!("Path is not a directory");
        }

        let mut manga_spec = base.clone();
        manga_spec.push("manga.json");

        if !manga_spec.is_file() {
            panic!("No manga spec");
        }

        let mut manga_info = base.clone();
        manga_info.push("series.json");

        if !manga_info.is_file() {
            panic!("No manga info");
        }

        let mut cover_path = base.clone();
        cover_path.push("cover.png");

        if !cover_path.is_file() {
            cover_path.set_extension("jpg");
        }

        if !cover_path.is_file() {
            panic!("No cover");
        }

        Paths {
            base,
            manga_spec,
            manga_info,
            cover_path,
        }
    }
}

fn main() {
    env_logger::init();

    let args = Args::parse();

    let server = server::Server::new(args.endpoint);

    let paths = Paths::new(args.path);

    info!("Manga Directory: {:?}", paths.base);
    info!("Manga Spec: {:?}", paths.manga_spec);
    info!("Manga Cover: {:?}", paths.cover_path);

    if args.update {
        info!("Running in update mode");
    }

    let manga_info = read_manga_info(&paths).unwrap();

    let manga = match server.get_manga(&manga_info.name) {
        Ok(manga) => manga,
        Err(Error::NoMangasWithName(_)) => {
            server.create_manga(&manga_info, paths.cover_path).unwrap()
        }
        Err(e) => panic!("Failed: {:?}", e),
    };

    let manga_chapters = server.get_chapters(&manga).unwrap();
    info!("{} chapters on the server", manga_chapters.len());
    // println!(
    //     "{:?}",
    //     manga_chapters.iter().map(|i| i.index).collect::<Vec<_>>()
    // );

    let local_chapters = get_local_chapters(&paths.base).unwrap();
    info!("{} chapters locally", local_chapters.len());
    // println!("Local: {:#?}", local_chapters);

    let mut missing_chapters = VecDeque::new();

    for local in local_chapters {
        // missing_chapters.push_back(local);
        let res = manga_chapters.iter().find(|i| i.index == local.index);
        if res.is_none() {
            missing_chapters.push_back(local);
        }
    }

    let num_missing_chapters = missing_chapters.len();

    info!("{} missing chapters", num_missing_chapters);

    if num_missing_chapters <= 0 {
        warn!("No chapters to process");
        println!("No chapters to process");
        return;
    }

    let mut num_threads = args.num_threads;
    if num_missing_chapters < num_threads {
        num_threads = num_missing_chapters;
    }

    info!("Using {} threads", num_threads);

    let work_queue = Arc::new(Mutex::new(missing_chapters));

    let mut thread_handles = Vec::new();
    for tid in 0..num_threads {
        let work_queue_handle = work_queue.clone();
        let s = server.clone();
        let m = manga.clone();
        let handle = std::thread::spawn(move || {
            worker_thread(tid, work_queue_handle, s, m);
        });
        thread_handles.push(handle);
    }

    loop {
        let left = {
            let lock =
                work_queue.lock().expect("Failed to get work queue lock");
            lock.len()
        };

        let num_done = num_missing_chapters - left;
        println!(
            "Num Done: {}",
            (num_done as f32 / num_missing_chapters as f32) * 100.0
        );
        std::thread::sleep(Duration::from_millis(750));

        if left <= 0 {
            break;
        }
    }

    for handle in thread_handles {
        handle.join().unwrap();
    }
}

fn get_pages_for_chapter<P>(missing: &LocalChapter, out_dir: P) -> Vec<String>
where
    P: AsRef<Path>,
{
    let path = out_dir.as_ref();
    let mut path = path.to_path_buf().clone();
    path.push(missing.index.to_string());

    let regex = Regex::new(r"page-(\d+).png").unwrap();

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

    pages.into_iter().map(|i| i.1).collect::<Vec<_>>()
}

fn is_already_processed<P>(out_dir: P, missing: &LocalChapter) -> bool
where
    P: AsRef<Path>,
{
    let mut path = out_dir.as_ref().to_path_buf();
    path.push(missing.index.to_string());

    path.is_dir()
}
