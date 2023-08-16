use std::{
    collections::VecDeque,
    fs::read_to_string,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};

// TODO(patrik):
//   - Create a verify process
//     - Check if the chapter dir is empty
//     - Check the server manga and the local manga should match
//     - Check server chapters vs local chapters
//   - Automate mangal
//     - mangas.json for all the mangas we have
//     - ability to add to mangas.json from a search function
//     - then download

use clap::{Parser, Subcommand};
use log::{debug, error, info, trace, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use server::{Manga, Server};

use crate::error::{Error, Result};

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
    publication_run: String,
}

#[derive(Debug)]
pub struct MangaInfo {
    name: String,
    mal_url: String,
    desc: String,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = false)]
    update: bool,

    /// Number of threads for processing chapters
    #[arg(short, long, default_value_t = 1)]
    num_threads: usize,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    UploadSingle {
        /// The pocketbase endpoint (example: http://localhost:8090)
        #[arg(short, long)]
        endpoint: String,

        path: PathBuf,
    },

    UploadMultiple {
        /// The pocketbase endpoint (example: http://localhost:8090)
        #[arg(short, long)]
        endpoint: String,

        dir: PathBuf,
    },

    AddManga {
        #[arg(short, long)]
        query: String,

        manga: PathBuf,
    },
}

fn read_manga_spec(paths: &Paths) -> Result<MangaSpec> {
    let s = read_to_string(&paths.manga_spec)
        .map_err(Error::ReadMangaSpecUnknown)?;
    serde_json::from_str::<MangaSpec>(&s)
        .map_err(|_| Error::InvalidMangaSpec(paths.manga_spec.clone()))
}

fn read_manga_info(paths: &Paths) -> Result<MangaInfo> {
    let spec = read_manga_spec(paths)?;

    let s = read_to_string(&paths.manga_info)
        .map_err(|_| Error::InvalidSeriesInfo(paths.manga_info.clone()))?;

    let v = serde_json::from_str::<serde_json::Value>(&s)
        .map_err(|_| Error::InvalidSeriesInfo(paths.manga_info.clone()))?;
    let v = &v["metadata"];
    let v = serde_json::from_value::<RawMangaInfo>(v.clone())
        .map_err(|_| Error::InvalidSeriesInfo(paths.manga_info.clone()))?;

    let desc = v.description_formatted;

    Ok(MangaInfo {
        name: spec.name.unwrap_or(v.name),
        mal_url: spec.mal_url,
        desc,
    })
}

#[derive(Clone, Debug)]
struct LocalChapter {
    index: usize,
    name: String,
    path: PathBuf,
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

    let regex =
        Regex::new(r"\[(\d+)\]_(Group_([\d.]+)_)*Chapter_([\d.]+)").ok()?;

    let mut res = Vec::new();
    for path in paths {
        let path = path.ok()?;
        let path = path.path();

        let filename = path.file_name()?.to_string_lossy();
        if let Some(captures) = regex.captures(&filename) {
            let index = captures[1].parse::<usize>().ok()?;
            let name = &captures[4];
            let group = if let Some(m) = captures.get(3) {
                m.as_str().parse::<usize>().unwrap()
            } else {
                1
            };

            let name = format!("Group {} - {}", group, name);

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

fn worker_thread(
    tid: usize,
    mangas: Arc<RwLock<VecDeque<PrepManga>>>,
    work_queue: Arc<Mutex<VecDeque<MissingChapter>>>,
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

        let lock =
            mangas.read().expect("Failed to aquire read lock on mangas");
        let manga = &lock[work.manga_index];
        let chapter = &manga.missing_chapters[work.chapter_index];

        trace!("{}: working on: {}-{}", tid, manga.info.name, chapter.index);

        let pages = get_chapter_pages(&chapter.path).unwrap();

        let _ = manga.server.add_chapter(
            &manga.manga,
            chapter.index,
            chapter.name.clone(),
            &pages,
        );
    }
}

#[derive(Debug)]
struct Paths {
    base: PathBuf,
    manga_spec: PathBuf,
    manga_info: PathBuf,
    cover_path: PathBuf,
}

impl Paths {
    fn new(base: PathBuf) -> Result<Self> {
        if !base.is_dir() {
            return Err(Error::PathNotDirectory(base));
        }

        let mut manga_info = base.clone();
        manga_info.push("series.json");

        if !manga_info.is_file() {
            return Err(Error::NoSeriesInfo(base));
        }

        let mut manga_spec = base.clone();
        manga_spec.push("manga.json");

        if !manga_spec.is_file() {
            return Err(Error::NoMangaSpec(base));
        }

        let mut cover_path = base.clone();
        cover_path.push("cover.png");

        if !cover_path.is_file() {
            cover_path.set_extension("jpg");
        }

        if !cover_path.is_file() {
            return Err(Error::NoCoverImage(base));
        }

        Ok(Paths {
            base,
            manga_spec,
            manga_info,
            cover_path,
        })
    }
}

#[derive(Debug)]
struct PrepManga {
    server: Server,
    paths: Paths,
    manga: Manga,
    info: MangaInfo,

    missing_chapters: VecDeque<LocalChapter>,
}

fn prep_manga<P>(path: P, endpoint: String) -> Result<PrepManga>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    let server = server::Server::new(endpoint);
    let paths = Paths::new(path.to_path_buf())?;

    trace!("Manga Directory: {:?}", paths.base);
    trace!("Manga Spec: {:?}", paths.manga_spec);
    trace!("Manga Cover: {:?}", paths.cover_path);

    let info = read_manga_info(&paths)?;

    let manga = match server.get_manga(&info.name) {
        Ok(manga) => Ok(manga),
        Err(Error::NoMangasWithName(_)) => {
            Ok(server.create_manga(&info, &paths.cover_path)?)
        }
        Err(e) => Err(Error::FailedToRetriveManga(Box::new(e))),
    }?;

    let manga_chapters = server.get_chapters(&manga)?;
    info!("{} chapters on the server", manga_chapters.len());

    let local_chapters = get_local_chapters(&paths.base)
        .ok_or(Error::FailedToGetLocalChapters)?;
    info!("{} chapters locally", local_chapters.len());

    let mut missing_chapters = VecDeque::new();

    for local in local_chapters {
        let res = manga_chapters.iter().find(|i| i.idx == local.index);
        if res.is_none() {
            missing_chapters.push_back(local);
        }
    }

    let num_missing_chapters = missing_chapters.len();

    info!("{} missing chapters", num_missing_chapters);

    Ok(PrepManga {
        server,
        paths,
        manga,
        info,

        missing_chapters,
    })
}

#[derive(Debug)]
struct MissingChapter {
    manga_index: usize,
    chapter_index: usize,
}

#[derive(Debug)]
struct MangalManga {
    name: String,
}

#[derive(Debug)]
struct AnilistManga {
    id: String,
}

fn extract_info_from_manga(value: &serde_json::Value) -> MangalManga {
    println!("Val: {:#?}", value);
    let mangal = value.get("mangal").expect("No mangal");
    let mangal = mangal.as_object().expect("Expected mangal to be an object");

    let name = mangal.get("name").expect("No name");
    let name = name.as_str().expect("Name is not a string").to_string();

    MangalManga { name }
}

fn extract_info_from_anilist(value: &serde_json::Value) -> AnilistManga {
    let id = value.get("id").expect("No id").to_string();

    AnilistManga { id }
}

fn query_mangas(query: &str) -> Vec<MangalManga> {
    // mangal inline -S Mangapill -q "Oshi no Ko" -j -a | jq | nvim -
    let output = Command::new("mangal")
        .arg("inline")
        .arg("-S")
        .arg("Mangapill")
        .arg("-q")
        .arg(query)
        .arg("-j")
        .output()
        .expect("Is 'mangal' installed?");

    println!("Status: {:?}", output.status);
    println!("Output: {:#?}", output);

    assert_eq!(output.stderr.len(), 0, "FIXME");

    let j =
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap();
    // println!("{:#?}", j);

    let results = j.get("result").expect("No result");
    assert!(results.is_array(), "'result' should be an array");

    let mut res = Vec::new();

    let results = results.as_array().unwrap();
    for result in results {
        // println!("Res: {:#?}", result);
        let manga = extract_info_from_manga(result);
        res.push(manga);
        // println!("Manga: {:#?}", manga_info);
    }

    res
}

fn query_anilist(query: &str) -> Vec<AnilistManga> {
    // mangal inline anilist search --name "the dangers in my heart" | jq | nvim

    let output = Command::new("mangal")
        .arg("inline")
        .arg("anilist")
        .arg("search")
        .arg("--name")
        .arg(query)
        .output()
        .expect("Is 'mangal' installed?");

    println!("Status: {:?}", output.status);
    println!("Output: {:#?}", output);

    assert_eq!(output.stderr.len(), 0, "FIXME");

    let j =
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap();
    println!("{:#?}", j);

    let mut res = Vec::new();

    let results = j.as_array().expect("Should be an array");
    for result in results {
        let manga = extract_info_from_anilist(result);
        res.push(manga);
    }

    res
}

fn main() {
    env_logger::init();

    let args = Args::parse();
    println!("Args: {:#?}", args);

    let mut mangas = VecDeque::new();

    let mut handle = |m: Result<PrepManga>| match m {
        Ok(manga) => {
            let num_missing = manga.missing_chapters.len();
            if num_missing > 0 {
                warn!(
                    "'{}' is missing {} chapter(s)",
                    manga.info.name, num_missing
                );
                mangas.push_back(manga);
            } else {
                debug!("'{}' not missing any chapters", manga.info.name);
            }
        }
        Err(Error::NoMangaSpec(path)) => {
            error!("{:?} is missing 'manga.json'", path)
        }
        Err(Error::NoSeriesInfo(path)) => {
            error!("{:?} is missing 'series.json'", path)
        }
        Err(Error::NoCoverImage(path)) => {
            error!("{:?} is missing 'cover[.png|.jpg]'", path)
        }
        Err(Error::InvalidMangaSpec(path)) => {
            error!("{:?} is a invalid 'manga.json'", path)
        }
        Err(Error::InvalidSeriesInfo(path)) => {
            error!("{:?} is a invalid 'series.json'", path)
        }
        Err(e) => error!("Unknown error: {:?}", e),
    };

    match args.command {
        Commands::UploadSingle { endpoint, path } => {
            let manga = prep_manga(path, endpoint);
            handle(manga);
        }
        Commands::UploadMultiple { endpoint, dir } => {
            let paths = dir.read_dir().unwrap();
            for path in paths {
                let path = path.unwrap();
                let path = path.path();

                trace!("Looking at {:?}", path);
                let manga = prep_manga(path, endpoint.clone());
                handle(manga);
            }
        }

        Commands::AddManga { query, manga: _ } => {
            let mangas = query_mangas(&query);
            let anilist = query_anilist(&query);

            panic!();
        }
    }

    if mangas.len() <= 0 {
        println!("Nothing to upload (exiting)");
        return;
    }

    println!("Num mangas to upload: {}", mangas.len());
    info!("-----------------");
    for manga in mangas.iter() {
        info!(
            "{} at {:?} needs to upload {} chapter(s)",
            manga.info.name,
            manga.paths.base,
            manga.missing_chapters.len()
        );
    }
    info!("-----------------");

    let total_missing_chapters = mangas
        .iter()
        .fold(0usize, |sum, val| sum + val.missing_chapters.len());
    debug!("Total missing chapters: {}", total_missing_chapters);

    let mut num_threads = args.num_threads;
    if total_missing_chapters < num_threads {
        num_threads = total_missing_chapters;
    }

    info!("Using {} threads", num_threads);

    let mut missing_chapters = VecDeque::new();
    for (manga_index, manga) in mangas.iter().enumerate() {
        for (chapter_index, _) in manga.missing_chapters.iter().enumerate() {
            missing_chapters.push_back(MissingChapter {
                manga_index,
                chapter_index,
            });
        }
    }

    // println!("Chapters: {:#?}", missing_chapters);

    let mangas = Arc::new(RwLock::new(mangas));
    let work_queue = Arc::new(Mutex::new(missing_chapters));

    let mut thread_handles = Vec::new();
    for tid in 0..num_threads {
        let work_queue_handle = work_queue.clone();
        let mangas_handle = mangas.clone();
        let handle = std::thread::spawn(move || {
            worker_thread(tid, mangas_handle, work_queue_handle);
        });
        thread_handles.push(handle);
    }

    loop {
        let left = {
            let lock =
                work_queue.lock().expect("Failed to get work queue lock");
            lock.len()
        };

        let num_done = total_missing_chapters - left;
        println!(
            "Num Done: {}",
            (num_done as f32 / total_missing_chapters as f32) * 100.0
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
