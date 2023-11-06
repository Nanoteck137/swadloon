use std::path::PathBuf;

use clap::{Parser, Subcommand};
use swadloon::{
    get_manga_chapters, get_manga_id, metadata_from_anilist,
    read_anilist_meta, write_manga_metadata,
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

fn main() {
    let args = Args::parse();
    println!("Args: {:#?}", args);

    // for path in args.path.read_dir().unwrap() {
    //     let path = path.unwrap();
    //     let path = path.path();
    //
    //     let mut p = path.clone();
    //     p.push("chapters");
    //
    //     for e in p.read_dir().unwrap() {
    //         let path = e.unwrap();
    //         let path = path.path();
    //
    //         let name = path.file_stem().unwrap().to_str().unwrap();
    //         let name = if let Some((name, _)) = name.split_once(": ") {
    //             name
    //         } else {
    //             println!("Skipping: {:?}", path);
    //             continue;
    //         };
    //
    //         let mut dest = path.clone();
    //         dest.set_file_name(name);
    //
    //         let src = path;
    //         println!("Dest: {:?}", dest);
    //         std::fs::rename(src, dest).unwrap();
    //     }
    // }
    //
    // panic!();

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

                        let arr = value.as_array().unwrap();

                        for chapter in arr {
                            let index = chapter
                                .get("index")
                                .unwrap()
                                .as_number()
                                .unwrap();
                            let name =
                                chapter.get("name").unwrap().as_str().unwrap();

                            println!("{} -> {}", index, name);

                            let mut src = chapters_dir.clone();
                            src.push(index.to_string());
                            src.push("name.txt");

                            std::fs::write(src, name).unwrap();
                        }
                    }
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
                write_manga_metadata(&path, &metadata);
            }
        }
    }
}
