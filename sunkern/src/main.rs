use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    dir: PathBuf,
}

fn main() {
    let args = Args::parse();
    println!("Args: {:#?}", args);
}
