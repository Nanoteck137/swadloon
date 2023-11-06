use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use swadloon::{
    anilist, download_image, gen_manga_id, ChapterMetadata, MangaMetadata,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    path: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Fix {
        #[arg(short, long)]
        refetch_anilist: bool,

        #[arg(short, long)]
        try_fix_chapter_name: bool,
    },
}

fn manga_dir<P>(base_dir: P, manga: &str) -> PathBuf
where
    P: AsRef<Path>,
{
    let mut p = base_dir.as_ref().to_path_buf();
    p.push(manga);

    p
}

fn read_manga_metadata<P>(manga_dir: P) -> MangaMetadata
where
    P: AsRef<Path>,
{
    let mut path = manga_dir.as_ref().to_path_buf();
    path.push("manga.json");

    let s = std::fs::read_to_string(path).unwrap();
    serde_json::from_str::<MangaMetadata>(&s).unwrap()
}

fn read_anilist_meta<P>(manga_dir: P) -> anilist::Metadata
where
    P: AsRef<Path>,
{
    let mut path = manga_dir.as_ref().to_path_buf();
    path.push("metadata.json");

    let s = std::fs::read_to_string(path).unwrap();
    serde_json::from_str::<anilist::Metadata>(&s).unwrap()
}

fn manga_image_dir<P>(manga_dir: P) -> PathBuf
where
    P: AsRef<Path>,
{
    let mut path = manga_dir.as_ref().to_path_buf();
    path.push("images");

    if !path.exists() {
        std::fs::create_dir(&path).unwrap();
    }

    path
}

fn metadata_from_anilist<P>(
    manga_dir: P,
    metadata: anilist::Metadata,
    id: String,
) -> MangaMetadata
where
    P: AsRef<Path>,
{
    let image_dir = manga_image_dir(manga_dir);
    let cover =
        download_image("cover", &metadata.cover_image.extra_large, &image_dir);
    let cover = cover.file_name().unwrap().to_str().unwrap().to_string();

    MangaMetadata {
        id,
        title: metadata.title.english.unwrap_or(metadata.title.romaji),
        cover,

        description: metadata.description,

        anilist_id: metadata.id,
        mal_id: metadata.mal_id.unwrap(),

        status: metadata.status,

        start_date: metadata.start_date.to_iso8601(),
        end_date: metadata.end_date.to_iso8601(),

        chapters: Vec::new(),
    }
}

fn get_manga_id<P>(manga_dir: P) -> String
where
    P: AsRef<Path>,
{
    let mut path = manga_dir.as_ref().to_path_buf();
    path.push("manga.json");

    println!("Path: {:?}", path);
    let s = std::fs::read_to_string(path).unwrap();
    let value = serde_json::from_str::<serde_json::Value>(&s).unwrap();

    if let Some(id) = value.get("id") {
        id.as_str().unwrap().to_string()
    } else {
        gen_manga_id()
    }
}

fn get_manga_chapters<P>(manga_dir: P, metadata: &mut MangaMetadata)
where
    P: AsRef<Path>,
{
    let mut chapters_dir = manga_dir.as_ref().to_path_buf();
    chapters_dir.push("chapters");

    for entry in chapters_dir.read_dir().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let file_stem = path.file_stem().unwrap().to_str().unwrap();

        let (chapter_index, chapter_name) =
            if let Some((index, name)) = file_stem.split_once(":") {
                let index = index.parse::<usize>().unwrap();
                (index, name)
            } else {
                let index = file_stem.parse::<usize>().unwrap();
                (index, file_stem)
            };

        let mut pages = path
            .read_dir()
            .unwrap()
            .map(|e| e.unwrap().path())
            .map(|e| {
                let file_name =
                    e.file_name().unwrap().to_str().unwrap().to_string();
                let (page_num, _) = file_name.split_once(".").unwrap();

                (page_num.parse::<usize>().unwrap(), file_name)
            })
            .collect::<Vec<_>>();
        pages.sort_by(|l, r| l.0.cmp(&r.0));

        let pages = pages.into_iter().map(|e| e.1).collect::<Vec<_>>();

        metadata.chapters.push(ChapterMetadata {
            index: chapter_index,
            name: format!("Chapter {}", chapter_name),
            pages,
        });
    }

    metadata.chapters.sort_by(|l, r| l.index.cmp(&r.index));
}

fn main() {
    let args = Args::parse();
    println!("Args: {:#?}", args);

    match args.command {
        Commands::Fix {
            refetch_anilist: _,
            try_fix_chapter_name,
        } => {
            for path in args.path.read_dir().unwrap() {
                let path = path.unwrap();
                let path = path.path();

                if !path.is_dir() {
                    continue;
                }

                println!("Fixing: {:?}", path);

                if try_fix_chapter_name {
                    let mut chapters_json = path.clone();
                    chapters_json.push("chapters.json");

                    let mut chapters_dir = path.clone();
                    chapters_dir.push("chapters");

                    if chapters_json.exists() {
                        let s =
                            std::fs::read_to_string(chapters_json).unwrap();
                        let value =
                            serde_json::from_str::<serde_json::Value>(&s)
                                .unwrap();
                        // println!("Value: {:#?}", value);

                        let arr = value.as_array().unwrap();

                        for chapter in arr {
                            let index = chapter
                                .get("index")
                                .unwrap()
                                .as_number()
                                .unwrap();
                            let name =
                                chapter.get("name").unwrap().as_str().unwrap();

                            let (_, name) = name
                                .split_once("Chapter ")
                                .expect("Unexpected format");

                            println!("{} -> {}", index, name);

                            let mut src = chapters_dir.clone();
                            src.push(index.to_string());

                            let mut dest = chapters_dir.clone();
                            dest.push(format!("{}: {}", index, name));

                            if dest.exists() {
                                println!("{:?}: Already fixed", dest);
                                continue;
                            }

                            std::fs::rename(src, dest).unwrap();
                        }
                    }

                    panic!();
                }

                let id = get_manga_id(&path);
                println!("Id: {}", id);

                let anilist_metadata = read_anilist_meta(&path);
                let mut metadata =
                    metadata_from_anilist(&path, anilist_metadata, id);

                get_manga_chapters(&path, &mut metadata);

                println!("Metadata: {:#?}", metadata);
                // let metadata = read_manga_metadata(path);
                // println!("Meta: {:#?}", metadata);
            }
        }
    }
}
