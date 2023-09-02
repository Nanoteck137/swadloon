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
    dir: PathBuf,
    endpoint: String,
}

fn main() {
    env_logger::init();

    let args = Args::parse();
    upload::upload(args.dir, args.endpoint, None)
}
