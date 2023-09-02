use std::path::PathBuf;

// TODO(patrik): List

use clap::{Parser, Subcommand};

mod shared;
mod error;
mod server;
mod upload;

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
}

fn main() {
    env_logger::init();

    let args = Args::parse();

    match args.command {
        Commands::Upload { endpoint, manga } => {
            upload::upload_new(args.dir, endpoint, manga)
        }
    }
}
