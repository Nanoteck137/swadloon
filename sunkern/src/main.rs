use std::{fs::File, io::Write, path::PathBuf, time::SystemTime};

use clap::{Parser, Subcommand};
use image::{GenericImage, ImageBuffer, RgbImage};
use swadloon::{anilist::fetch_anilist_metadata, server::Server};

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

// NOTE: Cover
// 460x657
// 460x718
// 460x715
// 400x636
// 460x723
// 460x720
// 460x654
// 460x653
// 460x723
// 460x650
// 460x717
// 460x722
// 460x654
// 460x654
// 460x643
// 460x654
// 460x650
// 460x634
// 460x652

// NOTE: Banner
// 1900x400
// 1900x400
// 1415x505
// 1900x400
// 1900x760
// 1900x400
// 1900x400
// 1899x399
// 1900x400
// 1900x400
// 1600x900
// 1900x400
// 1900x400
// 1900x492
// 1900x400
// 1900x626
// 1900x669
// 1900x400

fn hex(hex: u32) -> image::Rgb<u8> {
    let r = ((hex >> 16) & 0xff) as u8;
    let g = ((hex >> 8) & 0xff) as u8;
    let b = ((hex >> 0) & 0xff) as u8;
    image::Rgb([r, g, b])
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

            let mut image = RgbImage::new(460, 657);

            for y in 0..image.height() {
                for x in 0..image.width() {
                    image.put_pixel(x, y, image::Rgb([255, 0, 255]))
                }
            }

            image::imageops::vertical_gradient(
                &mut image,
                &hex(0xeb4034),
                &hex(0xeb9334),
            );

            let border_color = hex(0xeb348f);

            for y in 0..10 {
                for x in 0..image.width() {
                    image.put_pixel(x, y, border_color);
                    image.put_pixel(x, image.height() - 1 - y, border_color);
                }
            }

            for y in 0..image.height() {
                for x in 0..10 {
                    image.put_pixel(x, y, border_color);
                    image.put_pixel(image.width() - 1 - x, y, border_color);
                }
            }

            let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
            let t = time.as_micros();
            println!("T: {:?}", t);
            let name = format!("{}.png", t);
            println!("Name: {}", name);
            image.save(name).unwrap();

            for _ in 0..num {
                // let cover = gen_cover();
                // let manga = gen_manga();
            }
        }
    }
}
