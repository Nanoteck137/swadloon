use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, Duration},
};

use clap::{Parser, Subcommand};
use image::{GenericImage, ImageBuffer, RgbImage};
use rand::Rng;
use swadloon::{
    anilist::{
        fetch_anilist_metadata, Metadata, MetadataCoverImage, MetadataDate,
        MetadataTitle,
    },
    server::{ChapterMetadata, Server},
    ResolvedImages,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    AddMetadata { dir: PathBuf, mal_id: usize },

    GenerateTestData { endpoint: String, num: usize },
}

// NOTE: Names
const NAMES: &[&'static str] = &[
    "JoJo's Bizarre Adventure Part 9: The JoJoLands",
    "JoJo's Bizarre Adventure Part 8: Jojolion",
    "JoJo's Bizarre Adventure Part 7: Steel Ball Run",
    "JoJo's Bizarre Adventure Part 6: Stone Ocean",
    "JoJo's Bizarre Adventure Part 5: Golden Wind",
    "JoJo's Bizarre Adventure Part 4: Diamond Is Unbreakable",
    "JoJo's Bizarre Adventure Part 3: Stardust Crusaders",
    "JoJo's Bizarre Adventure Part 2: Battle Tendency",
    "JoJo's Bizarre Adventure Part 1: Phantom Blood",
    "The Quintessential Quintuplets",
    "Hell's Paradise: Jigokuraku",
    "Jujutsu Kaisen",
    "My Hero Academia",
    "One-Punch Man",
    "Tokyo Ghoul",
    "Chainsaw Man",
];

// NOTE: Cover
const COVERS: &[(usize, usize)] = &[
    (460, 657),
    (460, 718),
    (400, 636),
    (460, 720),
    (460, 654),
    (460, 723),
    (460, 650),
    (460, 722),
    (460, 654),
    (460, 654),
    (460, 643),
    (460, 650),
    (460, 634),
];

// NOTE: Banner
const BANNERS: &[(usize, usize)] = &[
    (1900, 400),
    (1415, 505),
    (1900, 760),
    (1899, 399),
    (1900, 400),
    (1600, 900),
    (1900, 492),
    (1900, 626),
    (1900, 669),
];

fn hex(hex: u32) -> image::Rgb<u8> {
    let r = ((hex >> 16) & 0xff) as u8;
    let g = ((hex >> 8) & 0xff) as u8;
    let b = ((hex >> 0) & 0xff) as u8;
    image::Rgb([r, g, b])
}

fn get_random_name() -> String {
    let index = rand::thread_rng().gen_range(0..NAMES.len());
    NAMES[index].to_string()
}

fn get_random_cover_size() -> (usize, usize) {
    let index = rand::thread_rng().gen_range(0..COVERS.len());
    COVERS[index]
}

fn get_random_banner_size() -> (usize, usize) {
    let index = rand::thread_rng().gen_range(0..BANNERS.len());
    BANNERS[index]
}

fn get_image<P>(cache_dir: P, size: (usize, usize)) -> PathBuf
where
    P: AsRef<Path>,
{
    let cache_dir = cache_dir.as_ref();

    let image = format!("{}x{}.png", size.0, size.1);
    let url = format!("https://dummyimage.com/{}/b942f5/fff", image);
    let mut output = cache_dir.to_path_buf();
    output.push(image);

    if !output.exists() {
        let status = Command::new("curl")
            .arg(url)
            .arg("--output")
            .arg(&output)
            .status();
        println!("Status: {:?}", status);
    } else {
        assert!(output.is_file());
    }

    output
}

fn main() {
    let args = Args::parse();
    println!("Args: {:#?}", args);

    match args.command {
        Commands::AddMetadata { dir, mal_id } => {
            println!("Adding metadata to {:?} with MalID '{}'", dir, mal_id);

            let mut metadata_file = dir.clone();
            metadata_file.push("metadata.json");

            if metadata_file.is_file() {
                // TODO(patrik): We should be able to override the
                // existing metadata
                panic!("Metadata file already exists");
            }

            let metadata = fetch_anilist_metadata(mal_id);
            let s = serde_json::to_string_pretty(&metadata).unwrap();
            let mut file = File::create(&metadata_file).unwrap();
            file.write_all(s.as_bytes()).unwrap();
        }

        Commands::GenerateTestData { endpoint, num } => {
            println!("Generating test data");

            let server = Server::new(endpoint);

            let mut cache_dir = dirs::cache_dir().unwrap();
            cache_dir.push(env!("CARGO_PKG_NAME"));
            println!("Cache Dir: {:?}", cache_dir);
            std::fs::create_dir_all(&cache_dir).unwrap();

            let mangas =
                server.get_all_manga().expect("Failed to get all mangas");
            println!("{} manga(s) exist on the server", mangas.len());

            for manga in mangas {
                server
                    .delete_manga(&manga.id)
                    .expect("Failed to delete manga");
            }

            let mut rng = rand::thread_rng();

            for i in 0..num {
                let name = if rng.gen_bool(2.0 / 3.0) {
                    get_random_name()
                } else {
                    let n = rng.gen_range(6..20);
                    lipsum::lipsum_with_rng(&mut rng, n)
                };
                let cover_size = get_random_cover_size();
                let banner_size = get_random_banner_size();

                // println!("{} {:?} {:?}", name, cover_size, banner_size);

                let cover_image = get_image(&cache_dir, cover_size);
                let banner_image = get_image(&cache_dir, banner_size);

                let n = rng.gen_range(6..=200);
                let description = lipsum::lipsum_with_rng(&mut rng, n);

                let metadata = Metadata {
                    id: 0,
                    mal_id: None,
                    title: MetadataTitle {
                        english: Some(name.to_string()),
                        native: None,
                        romaji: name.to_string(),
                    },
                    status: "".to_string(),

                    typ: "".to_string(),
                    format: "".to_string(),

                    description,
                    genres: Vec::new(),

                    chapters: None,
                    volumes: None,

                    banner_image: Some("".to_string()),
                    cover_image: MetadataCoverImage {
                        color: None,
                        medium: "".to_string(),
                        large: "".to_string(),
                        extra_large: "".to_string(),
                    },

                    start_date: MetadataDate {
                        day: Some(0),
                        month: Some(0),
                        year: Some(0),
                    },
                    end_date: MetadataDate {
                        day: Some(0),
                        month: Some(0),
                        year: Some(0),
                    },
                };

                let images = ResolvedImages {
                    banner: Some(banner_image),
                    cover_medium: cover_image.clone(),
                    cover_large: cover_image.clone(),
                    cover_extra_large: cover_image.clone(),
                };

                let metadata = (metadata, images).into();
                let manga = server
                    .create_manga(metadata)
                    .expect("Failed to create manga");

                for i in 0..rng.gen_range(0..=100) {
                    let n = rng.gen_range(6..20);
                    let name = lipsum::lipsum_with_rng(&mut rng, n);

                    let metadata = ChapterMetadata::new(
                        i,
                        name,
                        cover_image.clone(),
                        Vec::new(),
                    );

                    server
                        .add_chapter(&manga, metadata)
                        .expect("Failed to add chapter");
                }
            }
        }
    }
}
