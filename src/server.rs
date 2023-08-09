use std::path::Path;

use log::trace;
use reqwest::blocking::{multipart::Form, Client};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
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

#[derive(Clone)]
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

    pub fn get_manga(&self, name: &str) -> Result<Manga> {
        let filter = format!("(name~'{}')", name);
        let url = format!(
            "{}/api/collections/{}/records?filter={}",
            self.endpoint,
            MANGA_COLLECTION_NAME,
            urlencoding::encode(&filter)
        );
        trace!("get_manga: {}", url);

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

        let res = self
            .client
            .get(url)
            .send()
            .map_err(Error::FailedToSendRequest)?;

        let status = res.status();

        let res = res
            .json::<Result>()
            .map_err(Error::FailedToParseResponseJson)?;

        if res.total_items > 1 {
            return Err(Error::MoreThenOneManga);
        }

        if res.items.len() <= 0 {
            return Err(Error::NoMangasWithName(name.to_string()));
        }

        if status.is_success() {
            Ok(res.items[0].clone())
        } else {
            Err(Error::RequestFailed(status))
        }
    }

    pub fn create_manga<P>(
        &self,
        manga_spec: &MangaSpec,
        cover: P,
    ) -> Result<Manga>
    where
        P: AsRef<Path>,
    {
        let url = format!(
            "{}/api/collections/{}/records",
            self.endpoint, MANGA_COLLECTION_NAME,
        );
        trace!("create_manga (URL): {}", url);

        let form = Form::new()
            .text("name", manga_spec.name.clone())
            .text("malUrl", manga_spec.mal_url.clone())
            .file("cover", cover)
            .map_err(Error::FailedToIncludeFileInForm)?;

        let res = self
            .client
            .post(url)
            .multipart(form)
            .send()
            .map_err(Error::FailedToSendRequest)?;

        let status = res.status();

        if status.is_success() {
            let manga = res
                .json::<Manga>()
                .map_err(Error::FailedToParseResponseJson)?;

            Ok(manga)
        } else {
            Err(Error::RequestFailed(status))
        }
    }

    pub fn get_chapters(&self, manga: &Manga) -> Result<Vec<Chapter>> {
        let filter = format!("(manga~'{}')", manga.id);
        let url = format!(
            "{}/api/collections/{}/records?perPage=999&sort=index&filter={}",
            self.endpoint,
            CHAPTERS_COLLECTION_NAME,
            urlencoding::encode(&filter)
        );
        trace!("get_chapters: {}", url);

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

        let res = self
            .client
            .get(url)
            .send()
            .map_err(Error::FailedToSendRequest)?;

        let status = res.status();

        if status.is_success() {
            let res = res
                .json::<Result>()
                .map_err(Error::FailedToParseResponseJson)?;

            // TODO(patrik): This need to be fixed
            if res.total_pages > 1 {
                panic!("More then one page of chapters");
            }

            Ok(res.items)
        } else {
            Err(Error::RequestFailed(status))
        }
    }

    pub fn add_chapter(
        &self,
        manga: &Manga,
        index: usize,
        name: String,
        pages: &[String],
    ) -> Result<Chapter> {
        let url = format!(
            "{}/api/collections/{}/records",
            self.endpoint, CHAPTERS_COLLECTION_NAME,
        );
        trace!("add_chapter: {}", url);

        let mut form = Form::new()
            .text("index", index.to_string())
            .text("name", name.to_string())
            .text("manga", manga.id.clone());

        for page in pages {
            form = form
                .file("pages", page)
                .map_err(Error::FailedToIncludeFileInForm)?;
        }

        let res = self
            .client
            .post(url)
            .multipart(form)
            .send()
            .map_err(Error::FailedToSendRequest)?;

        let status = res.status();

        if status.is_success() {
            let res = res
                .json::<Chapter>()
                .map_err(Error::FailedToParseResponseJson)?;
            Ok(res)
        } else {
            Err(Error::RequestFailed(status))
        }
    }
}
