use std::path::PathBuf;

// TODO(patrik): List
//   - Create a verify process
//     - Check if the chapter dir is empty
//     - Check the server manga and the local manga should match
//     - Check server chapters vs local chapters
//   - Automate mangal
//     - mangas.json for all the mangas we have
//     - ability to add to mangas.json from a search function
//     - then download

use clap::{Parser, Subcommand};

mod shared;
mod error;
mod manga;
mod server;
mod upload;
mod util;

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

    Download {
        #[arg(short, long)]
        manga: Option<String>,
    },

    Process {
        #[arg(short, long)]
        manga: Option<String>,
    },
}

fn main() {
    env_logger::init();

    let args = Args::parse();
    println!("Args: {:#?}", args);

    match args.command {
        Commands::Upload { endpoint, manga } => {
            upload::upload_new(args.dir, endpoint, manga)
        }
        // Commands::AddManga { query } => manga::add_manga(args.dir, query),
        Commands::Download { manga } => manga::download_new(args.dir, manga),

        Commands::AddManga { query } => unimplemented!(),
        Commands::Process { manga } => unimplemented!(),
        // Commands::Process { manga } => process::process(args.dir, manga),
    }
}
