use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use log::{debug, error, trace};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::util;

#[derive(Serialize, Deserialize, Debug)]
struct MangaListEntry {
    id: String,
    anilist_id: String,
    name: String,
}

#[derive(Debug)]
struct MangalEntry {
    id: String,
    name: String,

    original: serde_json::Value,
}

#[derive(Debug)]
struct AnilistEntry {
    id: String,
    title: String,

    original: serde_json::Value,
}

fn extract_info_from_manga(value: &serde_json::Value) -> MangalEntry {
    // println!("Val: {:#?}", value);
    let mangal = value.get("mangal").expect("No mangal");
    let original = mangal.clone();
    let mangal = mangal.as_object().expect("Expected mangal to be an object");

    let name = mangal.get("name").expect("No name");
    let name = name.as_str().expect("Name is not a string").to_string();

    let id = name.to_lowercase();

    MangalEntry {
        id,
        name,
        original,
    }
}

fn query_mangal(query: &str) -> Vec<MangalEntry> {
    // mangal inline -S Mangapill -q "Oshi no Ko" -j
    let output = Command::new("mangal")
        .arg("inline")
        .arg("-S")
        .arg("Mangapill")
        .arg("-q")
        .arg(query)
        .arg("-j")
        .output()
        .expect("Is 'mangal' installed?");

    debug!("query_mangas: mangal exit code '{}'", output.status);

    assert_eq!(output.stderr.len(), 0, "FIXME");

    let j =
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap();

    let results = j.get("result").expect("No result");
    assert!(results.is_array(), "'result' should be an array");

    let mut entries = Vec::new();

    let results = results.as_array().unwrap();
    for result in results {
        let manga = extract_info_from_manga(result);
        entries.push(manga);
    }

    entries
}

fn extract_info_from_anilist(value: &serde_json::Value) -> AnilistEntry {
    let id = value.get("id").expect("No id").to_string();

    let title = value.get("title").expect("No title");
    let title = title.get("english").expect("No english title").to_string();

    AnilistEntry {
        id,
        title,
        original: value.clone(),
    }
}

fn query_anilist(query: &str) -> Vec<AnilistEntry> {
    // mangal inline anilist search --name "the dangers in my heart"

    let output = Command::new("mangal")
        .arg("inline")
        .arg("anilist")
        .arg("search")
        .arg("--name")
        .arg(query)
        .output()
        .expect("Is 'mangal' installed?");

    trace!("query_anilist: mangal exit status '{}'", output.status);

    assert_eq!(output.stderr.len(), 0, "FIXME");

    let j =
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap();

    let mut entries = Vec::new();

    let results = j.as_array().expect("Should be an array");
    for result in results {
        let manga = extract_info_from_anilist(result);
        entries.push(manga);
    }

    entries
}

fn read_index(prompt: &str) -> usize {
    print!("{}", prompt);
    std::io::stdout().flush().unwrap();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();

    input.trim().parse::<usize>().unwrap()
}

pub fn add_manga(dir: PathBuf, query: String) {
    trace!("add_manga: {}", query);

    let mangal_entries = query_mangal(&query);
    let anilist_entries = query_anilist(&query);

    for (index, manga) in mangal_entries.iter().enumerate() {
        println!("{} - {}", index, manga.name);
    }

    let mangal_index = read_index("Choose one manga: ");

    let mangal_entry = &mangal_entries[mangal_index];
    println!("Selected '{}'", mangal_entry.name);

    for (index, manga) in anilist_entries.iter().enumerate() {
        println!("{} - {}", index, manga.title);
    }

    let anilist_index = read_index("Link manga to anilist id: ");
    let anilist_entry = &anilist_entries[anilist_index];

    println!("Selected: {}", anilist_entry.title);

    let name = util::sanitize_name(&mangal_entry.name);

    let mut manga_json_file = dir.clone();
    manga_json_file.push("mangas.json");

    let mut entries = if manga_json_file.is_file() {
        let s = std::fs::read_to_string(&manga_json_file).unwrap();
        serde_json::from_str::<Vec<MangaListEntry>>(&s).unwrap()
    } else {
        Vec::new()
    };

    if let Some(_) = entries.iter().find(|i| i.id == mangal_entry.id) {
        println!("Entry already exists");
    } else {
        let new_entry = MangaListEntry {
            id: mangal_entry.id.clone(),
            anilist_id: anilist_entry.id.clone(),
            name: name.clone(),
        };

        entries.push(new_entry);
    }

    let s = serde_json::to_string_pretty(&entries).unwrap();
    let mut file = File::create(manga_json_file).unwrap();
    file.write_all(s.as_bytes()).unwrap();

    // TODO(patrik): Download covers

    let mut manga_dir = dir.clone();
    manga_dir.push(name);

    if !manga_dir.is_dir() {
        std::fs::create_dir(&manga_dir).unwrap();
    }

    let mut manga_json = manga_dir.clone();
    manga_json.push("manga.json");

    let mut file = File::create(manga_json).unwrap();
    let j = json!({
        "mangal": mangal_entry.original,
        "anilist": anilist_entry.original,
    });
    let s = serde_json::to_string_pretty(&j).unwrap();
    file.write_all(s.as_bytes()).unwrap();
}

fn read_manga_list<P>(dir: P) -> Vec<MangaListEntry>
where
    P: AsRef<Path>,
{
    let mut path = dir.as_ref().to_path_buf();
    path.push("mangas.json");

    if !path.is_file() {
        panic!("Missing 'mangas.json' is manga directory");
    }

    let s = std::fs::read_to_string(path).unwrap();

    serde_json::from_str::<Vec<MangaListEntry>>(&s).unwrap()
}

fn download_single<P>(dir: P, entry: &MangaListEntry)
where
    P: AsRef<Path>,
{
    let mut output_dir = dir.as_ref().to_path_buf();
    output_dir.push(&entry.name);
    output_dir.push("chapters");
    debug!("Trying to download '{}' -> {:?}", entry.id, output_dir);

    if !output_dir.is_dir() {
        std::fs::create_dir(&output_dir).unwrap();
    }

    // mangal inline -S Mangapill -q "Oshi no Ko" -m first -d

    // TODO(patrik): Maybe we could use the series info to see if we downloaded 
    // the right one

    let status = Command::new("mangal")
        .env("MANGAL_METADATA_SERIES_JSON", "false")
        .env("MANGAL_FORMATS_USE", "zip")
        .env("MANGAL_DOWNLOADER_PATH", output_dir)
        .env("MANGAL_DOWNLOADER_DOWNLOAD_COVER", "false")
        .env("MANGAL_DOWNLOADER_CREATE_MANGA_DIR", "false")
        .arg("inline")
        .arg("-S")
        .arg("Mangapill")
        .arg("-q")
        .arg(&entry.id)
        .arg("-m")
        .arg("first")
        .arg("-d")
        .status()
        .expect("Is 'mangal' installed?");

    assert!(status.success());

    // assert_eq!(output.stderr.len(), 0, "FIXME");
}

pub fn download(dir: PathBuf, manga: Option<String>) {
    trace!("download: {:?}", manga);

    let manga_list = read_manga_list(&dir);

    if let Some(manga) = manga {
        // NOTE(patrik): Just download a single entry inside the list
        if let Some(entry) = manga_list.iter().find(|i| i.id == manga) {
            download_single(&dir, entry);
        } else {
            error!("'{}' is not inside the manga list", manga);
        }
    } else {
        // NOTE(patrik): Download all mangas in the list
        for entry in manga_list {
            download_single(&dir, &entry);
        }
    }
}
