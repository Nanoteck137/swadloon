use std::path::Path;

use reqwest::blocking::{multipart::Form, Client};
use serde::{Deserialize, Serialize};

use crate::MangaSpec;

const MANGA_COLLECTION_NAME: &str = "manga";
const CHAPTERS_COLLECTION_NAME: &str = "chapters";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Manga {
    pub id: String,
    pub name: String,
    pub cover: String,
    #[serde(rename = "malUrl")]
    pub mal_url: String,

    pub created: String,
    pub updated: String,

    #[serde(rename = "collectionId")]
    pub collection_id: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Chapter {
    pub id: String,
    pub index: usize,
    pub name: String,
    pub manga: String,
    pub pages: Vec<String>,

    pub created: String,
    pub updated: String,

    #[serde(rename = "collectionId")]
    pub collection_id: String,
    #[serde(rename = "collectionName")]
    pub collection_name: String,
}

pub struct Server {
    endpoint: String,
    client: Client,
}

impl Server {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            client: Client::new(),
        }
    }

    pub fn get_manga(&self, name: &str) -> Option<Manga> {
        let filter = format!("(name~'{}')", name);
        let url = format!(
            "{}/api/collections/{}/records?filter={}",
            self.endpoint,
            MANGA_COLLECTION_NAME,
            urlencoding::encode(&filter)
        );
        println!("URL: {}", url);

        #[derive(Deserialize, Debug)]
        struct Result {
            items: Vec<Manga>,
            // page: usize,
            // #[serde(rename = "perPage")]
            // per_page: usize,
            #[serde(rename = "totalItems")]
            total_items: usize,
            // #[serde(rename = "totalPages")]
            // total_pages: usize,
        }

        let res = self.client.get(url).send().ok()?;
        let status = res.status();
        let j = res.json::<Result>().unwrap();

        if j.total_items > 1 {
            panic!("More then one item???");
        }

        if status.is_success() && j.items.len() == 1 {
            Some(j.items[0].clone())
        } else {
            None
        }
    }

    pub fn create_manga<P>(
        &self,
        manga_spec: &MangaSpec,
        cover: P,
    ) -> Option<Manga>
    where
        P: AsRef<Path>,
    {
        let url = format!(
            "{}/api/collections/{}/records",
            self.endpoint, MANGA_COLLECTION_NAME,
        );
        println!("URL: {}", url);

        let form = Form::new()
            .text("name", manga_spec.name.clone())
            .text("malUrl", manga_spec.mal_url.clone())
            .file("cover", cover)
            .unwrap();

        let res = self.client.post(url).multipart(form).send().ok()?;
        let manga = res.json::<Manga>().ok()?;

        Some(manga)
    }

    pub fn get_chapters(&self, manga: &Manga) -> Option<Vec<Chapter>> {
        let filter = format!("(manga~'{}')", manga.id);
        let url = format!(
            "{}/api/collections/{}/records?perPage=999&sort=index&filter={}",
            self.endpoint,
            CHAPTERS_COLLECTION_NAME,
            urlencoding::encode(&filter)
        );
        println!("URL: {}", url);

        #[derive(Deserialize, Debug)]
        struct Result {
            items: Vec<Chapter>,
            // page: usize,
            // #[serde(rename = "perPage")]
            // per_page: usize,
            // #[serde(rename = "totalItems")]
            // total_items: usize,
            #[serde(rename = "totalPages")]
            total_pages: usize,
        }

        let res = self.client.get(url).send().ok()?;
        let status = res.status();
        let j = res.json::<Result>().unwrap();

        // TODO(patrik): This need to be fixed
        if j.total_pages > 1 {
            panic!("More then one page of chapters");
        }

        if status.is_success() {
            Some(j.items)
        } else {
            None
        }
    }

    pub fn add_chapter(
        &self,
        manga: &Manga,
        index: usize,
        name: String,
        pages: &[String],
    ) -> Option<Chapter> {
        let url = format!(
            "{}/api/collections/{}/records",
            self.endpoint, CHAPTERS_COLLECTION_NAME,
        );
        println!("URL: {}", url);

        let mut form = Form::new()
            .text("index", index.to_string())
            .text("name", name.to_string())
            .text("manga", manga.id.clone());

        for page in pages {
            form = form.file("pages", page).unwrap();
        }

        let res = self.client.post(url).multipart(form).send().ok()?;
        let status = res.status();

        if status.is_success() {
            let res = res.json::<Chapter>().ok()?;
            Some(res)
        } else {
            None
        }
    }
}
