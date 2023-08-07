use std::path::{Path, PathBuf};

use reqwest::blocking::multipart::Form;
use serde::{Serialize, Deserialize};

#[derive(Debug)]
struct Manga {
    name: String,
    mal_url: String,
    thumbnail: PathBuf,
}

impl Manga {
    fn to_form(&self) -> Form {
        Form::new()
            .text("name", self.name.clone())
            .text("malUrl", self.mal_url.clone())
            .file("thumbnail", self.thumbnail.clone())
            .unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct MangaResponse {
    #[serde(rename = "collectionId")]
    collection_id: String,
    #[serde(rename = "collectionName")]
    collection_name: String,

    id: String,
    #[serde(rename = "malUrl")]
    mal_url: String,
    name: String,
    thumbnail: String,

    created: String,
    updated: String,
}

fn create_manga(manga: &Manga) -> Option<MangaResponse> {
    let client = reqwest::blocking::Client::new();

    let form = manga.to_form();

    let collection = "manga";
    let res = client
        .post(format!(
            "http://127.0.0.1:8090/api/collections/{}/records",
            collection
        ))
        .multipart(form)
        .send()
        .unwrap();

    if res.status().is_success() {
        res.json::<MangaResponse>().ok()
    } else {
        None
    }
}

fn main() {
    let manga = Manga {
        name: "Oshi no Ko".to_string(),
        mal_url: "https://myanimelist.net/manga/126146/Oshi_no_Ko".to_string(),
        thumbnail: PathBuf::from("/home/nanoteck137/wallpaper.png"),
    };

    // let res = create_manga(&manga);
    // println!("Res: {:#?}", res);
    // return;
    
    let client = reqwest::blocking::Client::new();

    let form = Form::new()
        .text("num", "0")
        .text("name", "Wot")
        .text("manga", "18f6sxhqycha8z9")
        .file("pages", "/home/nanoteck137/p/page-000.png").unwrap()
        .file("pages", "/home/nanoteck137/p/page-001.png").unwrap()
        .file("pages", "/home/nanoteck137/p/page-002.png").unwrap()
        .file("pages", "/home/nanoteck137/p/page-003.png").unwrap()
        .file("pages", "/home/nanoteck137/p/page-004.png").unwrap()
        .file("pages", "/home/nanoteck137/p/page-005.png").unwrap()
        ;

    let collection = "chapters";
    let res = client
        .post(format!(
            "http://127.0.0.1:8090/api/collections/{}/records",
            collection
        ))
        .multipart(form)
        .send()
        .unwrap();
    println!("Res: {:#?}", res.json::<serde_json::Value>());
}
