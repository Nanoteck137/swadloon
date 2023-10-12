use std::path::PathBuf;

// TODO(patrik): List
//  - Add indication of full update
//  - Add better progress bar

use clap::Parser;

mod error;
mod upload;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    dir: PathBuf,
    endpoint: String,

    #[arg(short, long)]
    full_update: bool,
}

fn main() {
    env_logger::init();

    let args = Args::parse();
    upload::upload(args.dir, args.endpoint, args.full_update)
}
