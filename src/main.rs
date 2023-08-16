use std::{
    collections::VecDeque,
    fs::{read_to_string, File},
    io::Write,
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
use serde_json::json;
use server::{Manga, Server};

use crate::error::{Error, Result};

mod error;
mod server;
mod upload;
mod manga;
mod util;

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
    #[arg(short, long, default_value_t = 1)]
    num_threads: usize,

    dir: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Upload {
        endpoint: String,

        #[arg(short, long)]
        manga: Option<String>,
    },

    AddManga {
        query: String,
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


fn main() {
    env_logger::init();

    let args = Args::parse();
    println!("Args: {:#?}", args);

    match args.command {
        Commands::Upload { endpoint, manga } => upload::upload(endpoint, manga),
        Commands::AddManga { query } => manga::add_manga(args.dir, query),

        //
        // Commands::AddManga { query, dir } => {
        //     let mangas = query_mangas(&query);
        //     let anilist = query_anilist(&query);
        //
        //     for (index, manga) in mangas.iter().enumerate() {
        //         println!("{} - {}", index, manga.0.name);
        //     }
        //
        //     let read_index = |prompt: &str| {
        //         print!("{}", prompt);
        //         std::io::stdout().flush().unwrap();
        //         let mut input = String::new();
        //         std::io::stdin().read_line(&mut input).unwrap();
        //
        //         input.trim().parse::<usize>().unwrap()
        //     };
        //
        //     let index = read_index("Choose one manga: ");
        //
        //     let (mangal_manga, mangal_value) = &mangas[index];
        //     println!("Selected '{}'", mangal_manga.name);
        //
        //     for (index, manga) in anilist.iter().enumerate() {
        //         println!("{} - {}", index, manga.0.title);
        //     }
        //
        //     let anilist_index = read_index("Link manga to anilist id: ");
        //     let (anilist_manga, anilist_value) = &anilist[anilist_index];
        //
        //     println!("Selected: {}", anilist_manga.title);
        //
        //     let name = sanitize_name(&mangal_manga.name);
        //
        //     #[derive(Serialize, Deserialize, Debug)]
        //     struct Entry {
        //         id: String,
        //         anilist_id: String,
        //         name: String,
        //     }
        //
        //     let mut manga_json_file = dir.clone();
        //     manga_json_file.push("mangas.json");
        //
        //     let mut entries = if manga_json_file.is_file() {
        //         let s = read_to_string(&manga_json_file).unwrap();
        //         serde_json::from_str::<Vec<Entry>>(&s).unwrap()
        //     } else {
        //         Vec::new()
        //     };
        //
        //     if let Some(_) = entries.iter().find(|i| i.id == mangal_manga.name)
        //     {
        //         println!("Entry already exists");
        //     } else {
        //         let new_entry = Entry {
        //             id: mangal_manga.name.clone(),
        //             anilist_id: anilist_manga.id.clone(),
        //             name: name.clone(),
        //         };
        //
        //         entries.push(new_entry);
        //     }
        //
        //     let s = serde_json::to_string_pretty(&entries).unwrap();
        //     let mut file = File::create(manga_json_file).unwrap();
        //     file.write_all(s.as_bytes()).unwrap();
        //
        //     let mut manga_dir = dir.clone();
        //     manga_dir.push(name);
        //
        //     if !manga_dir.is_dir() {
        //         std::fs::create_dir(&manga_dir).unwrap();
        //     }
        //
        //     let mut manga_json = manga_dir.clone();
        //     manga_json.push("manga.json");
        //
        //     let mut file = File::create(manga_json).unwrap();
        //     let j = json!({
        //         "mangal": mangal_value,
        //         "anilist": anilist_value,
        //     });
        //     let s = serde_json::to_string_pretty(&j).unwrap();
        //     file.write_all(s.as_bytes()).unwrap();
        // }
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
