use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use log::{debug, error, info, trace};
use regex::Regex;
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::manga::{read_manga_list, MangaListEntry};

#[derive(Serialize, Deserialize, Debug)]
pub struct MangaImages {
    pub banner: String,
    pub cover_medium: String,
    pub cover_large: String,
    pub cover_extra_large: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MangaMetadata {
    pub name: String,

    pub english_title: String,
    pub native_title: String,
    pub romaji_title: String,

    pub anilist_url: String,
    pub mal_url: String,

    pub description: String,

    pub start_date: String,
    pub end_date: String,

    pub color: String,
    pub is_group: bool,

    pub images: MangaImages,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChapterMetadata {
    pub index: usize,
    pub name: String,
    pub group: usize,
    pub pages: Vec<String>,
}

fn process_meta(
    dir: &PathBuf,
    output_dir: &PathBuf,
    name: &str,
    is_group: bool,
) {
    let mut metadata_file = dir.clone();
    metadata_file.push("manga.json");

    if !metadata_file.is_file() {
        panic!("No 'manga.json' inside '{:?}'", metadata_file);
    }

    let mut out = output_dir.clone();
    out.push("manga.json");

    info!("Processing metadata '{}'", name,);

    let s = std::fs::read_to_string(&metadata_file).unwrap();
    let j = serde_json::from_str::<serde_json::Value>(&s).unwrap();

    let anilist = j.get("anilist").expect("No anilist");

    let title = anilist.get("title").expect("No title");
    let english_title = title.get("english").expect("No english");
    let native_title = title.get("native").expect("No native");
    let romaji_title = title.get("romaji").expect("No romaji");

    let site_url = anilist
        .get("siteUrl")
        .expect("No siteUrl")
        .as_str()
        .expect("Expected siteUrl to be an string");
    let mal_id = anilist
        .get("idMal")
        .expect("No idMal")
        .as_u64()
        .expect("Expected idMal to be an integer");

    let desc = anilist.get("description").expect("No description");

    let parse_date = |date: &serde_json::Value| {
        let year = date
            .get("year")
            .expect("No year")
            .as_u64()
            .expect("year should be an integer");
        let month = date
            .get("month")
            .expect("No month")
            .as_u64()
            .expect("month should be an integer");
        let day = date
            .get("day")
            .expect("No day")
            .as_u64()
            .expect("day should be an integer");

        let date = if year > 0 && month > 0 && day > 0 {
            format!("{}-{:02}-{:02}", year, month, day)
        } else {
            "".to_string()
        };

        date
    };

    let start_date = anilist.get("startDate").expect("No startDate");
    let start_date = parse_date(start_date);

    let end_date = anilist.get("endDate").expect("No endDate");
    let end_date = parse_date(end_date);

    let process_image = |name: &str, url: &str| -> String {
        let url_filename = url.split("/").last().unwrap();
        let url_file_ext = url_filename.split(".").last().unwrap();

        let mut out = output_dir.clone();
        out.push(name);
        out.set_extension(url_file_ext);

        let file_name = out.file_name().unwrap().to_str().unwrap().to_string();

        if out.is_file() {
            debug!("Skipping downloading: {:?}", out);
            return file_name;
        }

        // FIXME(patrik): Why do we need the -k
        let status = Command::new("curl")
            .arg("-k")
            .arg(url)
            .arg("--output")
            .arg(&out)
            .status()
            .unwrap();
        if !status.success() {
            panic!("Failed to download image '{}'", url);
        }

        return file_name;
    };

    let cover_image = anilist.get("coverImage").expect("No coverImage");
    let color = cover_image.get("color").expect("No color");

    let extra_large = cover_image
        .get("extraLarge")
        .expect("No extraLarge")
        .as_str()
        .expect("extraLarge should be an string");
    let large = cover_image
        .get("large")
        .expect("No large")
        .as_str()
        .expect("large should be an string");
    let medium = cover_image
        .get("medium")
        .expect("No medium")
        .as_str()
        .expect("medium should be an string");

    let cover_medium = process_image("medium", medium);
    let cover_large = process_image("large", large);
    let cover_extra_large = process_image("extra_large", extra_large);

    let banner_image = anilist
        .get("bannerImage")
        .expect("No bannerImage")
        .as_str()
        .expect("bannerImage should be an string");

    let banner = process_image("banner_image", banner_image);

    let images = MangaImages {
        banner,
        cover_medium,
        cover_large,
        cover_extra_large,
    };

    let mal_url = format!("https://myanimelist.net/manga/{}", mal_id);

    let metadata = MangaMetadata {
        name: name.to_string(),
        english_title: english_title.to_string(),
        native_title: native_title.to_string(),
        romaji_title: romaji_title.to_string(),

        anilist_url: site_url.to_string(),
        mal_url,

        description: desc.to_string(),

        start_date,
        end_date,

        color: color.to_string(),
        is_group,

        images,
    };

    let s = serde_json::to_string_pretty(&metadata).unwrap();
    let mut file = File::create(&out).unwrap();
    file.write_all(s.as_bytes()).unwrap();
}

fn process_chapters(chapters_dir: &PathBuf, output_dir: &PathBuf) -> bool {
    // TODO(patrik): Check for corrupt chapters

    let regex =
        Regex::new(r"\[(\d+)\]_(Group_([\d.]+)_)*Chapter_([\d.]+)").unwrap();

    #[derive(Debug)]
    struct Chapter {
        path: PathBuf,

        index: usize,
        group: usize,
        name: String,
    }

    let mut chapters = Vec::new();

    let mut is_group = false;

    let paths = chapters_dir.read_dir().unwrap();
    for path in paths {
        let path = path.unwrap();
        let path = path.path();

        let filename = path.file_stem().unwrap().to_string_lossy();
        if let Some(cap) = regex.captures(&filename) {
            let index = cap[1].parse::<usize>().unwrap();
            let group = cap
                .get(3)
                .map(|i| i.as_str().parse::<usize>().unwrap())
                .unwrap_or(0);
            if group != 0 {
                is_group = true;
            }
            let name = cap[4].to_string();

            chapters.push(Chapter {
                path,
                index,
                group,
                name,
            });
        }
    }

    chapters.sort_by(|l, r| l.index.cmp(&r.index));

    for chapter in chapters {
        let mut out = output_dir.clone();
        out.push(chapter.index.to_string());

        if !out.is_dir() {
            std::fs::create_dir(&out).unwrap();
        }

        let mut chapter_info = out.clone();
        chapter_info.push("info.json");

        if chapter_info.is_file() {
            debug!("Skipping '{}' because 'info.json' exists", chapter.index);
            continue;
        }

        let file = File::open(chapter.path).unwrap();
        let mut zip = ZipArchive::new(file).unwrap();

        let num_pages = zip.len();

        info!("Working on {} - {} pages", chapter.index, num_pages);

        // TODO(patrik): If we got error we should report those errors because
        // it's likely a currupt zip file
        zip.extract(out).unwrap();

        let mut pages = zip
            .file_names()
            .map(|i| {
                let index = i.split(".").next().unwrap();
                let index = index.parse::<usize>().unwrap();
                (index, i)
            })
            .collect::<Vec<_>>();

        pages.sort_by(|l, r| l.0.cmp(&r.0));

        let pages = pages.iter().map(|i| i.1.to_string()).collect::<Vec<_>>();

        let metadata = ChapterMetadata {
            index: chapter.index,
            name: chapter.name,
            group: chapter.group,
            pages,
        };

        let s = serde_json::to_string_pretty(&metadata).unwrap();
        let mut file = File::create(chapter_info).unwrap();
        file.write_all(s.as_bytes()).unwrap();
    }

    is_group
}

fn process_single<P>(dir: P, entry: &MangaListEntry)
where
    P: AsRef<Path>,
{
    debug!("Trying to process '{}'", entry.id);

    let dir = dir.as_ref().to_path_buf();

    let mut entry_dir = dir.clone();
    entry_dir.push(&entry.name);

    let mut chapter_dir = entry_dir.clone();
    chapter_dir.push("chapters");
    debug!("Chapter Dir: {:?}", chapter_dir);

    let mut output_dir = entry_dir.clone();
    output_dir.push("processed");

    let mut chapter_output_dir = output_dir.clone();
    chapter_output_dir.push("chapters");

    if !output_dir.is_dir() {
        std::fs::create_dir(&output_dir).unwrap();
    }

    if !chapter_output_dir.is_dir() {
        std::fs::create_dir(&chapter_output_dir).unwrap();
    }

    let is_group = process_chapters(&chapter_dir, &chapter_output_dir);
    process_meta(&entry_dir, &output_dir, &entry.id, is_group);
}

pub fn process(dir: PathBuf, manga: Option<String>) {
    let manga_list = read_manga_list(&dir);

    if let Some(manga) = manga {
        // NOTE(patrik): Process single manga
        if let Some(entry) = manga_list.iter().find(|i| i.id == manga) {
            process_single(&dir, entry);
        } else {
            error!("No manga inside list: {}", manga);
        }
    } else {
        // NOTE(patrik): Process all the mangas inside the list
        for entry in manga_list {
            process_single(&dir, &entry);
        }
    }
}
