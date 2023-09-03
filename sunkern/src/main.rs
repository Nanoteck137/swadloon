use std::{fs::File, io::Write, path::PathBuf};

use clap::{Parser, Subcommand};
use swadloon::anilist::fetch_anilist_metadata;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    AddMetadata { dir: PathBuf, mal_id: usize },
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
    }
}
