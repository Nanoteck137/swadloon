use std::{
    collections::VecDeque,
    fs::read_to_string,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};

use clap::Parser;
use log::{debug, info, trace};
use regex::Regex;
use serde::{Deserialize, Serialize};
use server::{Manga, Server};

use crate::error::Error;

mod error;
mod server;

#[derive(Serialize, Deserialize, Debug)]
pub struct MangaSpec {
    name: String,
    #[serde(rename = "malUrl")]
    mal_url: String,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The pocketbase endpoint (example: http://localhost:8090)
    endpoint: String,
    /// Path to the manga directory (i.e. where the 'manga.json' is)
    path: PathBuf,
    /// Path to the output directory where all the processed chapters will be stored
    out_dir: PathBuf,

    /// Number of threads for processing chapters
    #[arg(short, long, default_value_t = 1)]
    num_threads: usize,
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

fn get_local_chapters<P>(path: P) -> Option<Vec<LocalChapter>>
where
    P: AsRef<Path>,
{
    let paths = std::fs::read_dir(path).ok()?;

    let regex = Regex::new(r"\[(\d+)\]_\w+_([\d.]+).pdf").unwrap();

    let mut res = Vec::new();
    for path in paths {
        let path = path.ok()?;
        let path = path.path();

        let filename = path.file_name()?.to_string_lossy();
        if let Some(captures) = regex.captures(&filename) {
            let index = captures[1].parse::<usize>().ok()?;
            let name = &captures[2];

            res.push(LocalChapter {
                index,
                name: name.to_string(),
                path,
            });
        }
    }

    res.sort_by(|l, r| l.index.cmp(&r.index));

    Some(res)
}

fn worker_thread<P>(
    tid: usize,
    work_queue: Arc<Mutex<VecDeque<LocalChapter>>>,
    out_dir: P,
    server: Server,
    manga: Manga,
) where
    P: AsRef<Path>,
{
    let out_dir = out_dir.as_ref();

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

        // TODO(patrik): Move to path and remove is_already_processed
        if is_already_processed(&out_dir, &work) {
            info!("{} is already processed", work.index);
        } else {
            let mut p = out_dir.to_path_buf();
            p.push(work.index.to_string());
            std::fs::create_dir(&p).unwrap();

            process_chapter(&work, p);
        }

        let pages = get_pages_for_chapter(&work, &out_dir);
        server
            .add_chapter(&manga, work.index, work.name, &pages)
            .unwrap();
    }
}

fn main() {
    env_logger::init();

    let args = Args::parse();

    let server = server::Server::new(args.endpoint);

    let path = args.path;

    if !path.is_dir() {
        panic!("Path is not a directory");
    }

    let mut manga_spec = path.clone();
    manga_spec.push("manga.json");

    let mut cover_path = path.clone();
    cover_path.push("cover.png");

    if !cover_path.is_file() {
        cover_path.set_extension("jpg");
    }

    if !cover_path.is_file() {
        panic!("No cover");
    }

    let name = path.file_name().unwrap();

    let mut out_dir = args.out_dir;
    out_dir.push(name);

    if !out_dir.is_dir() {
        std::fs::create_dir_all(&out_dir).unwrap();
    }

    let manga_spec = read_manga_spec(manga_spec).unwrap();
    let manga = match server.get_manga(&manga_spec.name) {
        Ok(manga) => manga,
        Err(Error::NoMangasWithName(_)) => {
            server.create_manga(&manga_spec, cover_path).unwrap()
        }
        Err(_) => panic!("Failed"),
    };

    let manga_chapters = server.get_chapters(&manga).unwrap();
    info!("{} chapters on the server", manga_chapters.len());
    // println!(
    //     "{:?}",
    //     manga_chapters.iter().map(|i| i.index).collect::<Vec<_>>()
    // );

    let local_chapters = get_local_chapters(path).unwrap();
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
        let o = out_dir.clone();
        let s = server.clone();
        let m = manga.clone();
        let handle = std::thread::spawn(move || {
            worker_thread(tid, work_queue_handle, o, s, m);
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

fn process_chapter<P>(chapter: &LocalChapter, path: P)
where
    P: AsRef<Path>,
{
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
