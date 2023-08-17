use std::path::{Path, PathBuf};

use log::{debug, trace};
use reqwest::blocking::ClientBuilder;
use reqwest::blocking::{multipart::Form, Client};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::MangaInfo;
use crate::process::MangaMetadata;

const MANGA_COLLECTION_NAME: &str = "mangas";
const CHAPTERS_COLLECTION_NAME: &str = "chapters";



#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Manga {
    pub id: String,
    pub name: String,

    #[serde(rename = "englishTitle")]
    pub english_title: String,
    #[serde(rename = "nativeTitle")]
    pub native_title: String,
    #[serde(rename = "romajiTitle")]
    pub romaji_title: String,

    #[serde(rename = "malUrl")]
    pub mal_url: String,
    #[serde(rename = "anilistUrl")]
    pub anilist_url: String,

    pub description: String,
    #[serde(rename = "isGroup")]
    pub is_group: bool,

    pub banner: String,
    #[serde(rename = "coverMedium")]
    pub cover_medium: String,
    #[serde(rename = "coverLarge")]
    pub cover_large: String,
    #[serde(rename = "coverExtraLarge")]
    pub cover_extra_large: String,

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
    pub idx: usize,
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

#[derive(Deserialize, Debug)]
struct ChapterPage {
    items: Vec<Chapter>,
    page: usize,
    // #[serde(rename = "perPage")]
    // per_page: usize,
    #[serde(rename = "totalItems")]
    total_items: usize,
    #[serde(rename = "totalPages")]
    total_pages: usize,
}

pub fn create_form_from_metadata() {
}

#[derive(Clone, Debug)]
pub struct Server {
    endpoint: String,
    client: Client,
}

impl Server {
    pub fn new(endpoint: String) -> Self {
        let client = ClientBuilder::new().timeout(None).build().unwrap();

        Self { endpoint, client }
    }

    pub fn get_manga(&self, name: &str) -> Result<Manga> {
        let filter = format!("(name='{}')", name);

        let url = format!(
            "{}/api/collections/{}/records?filter={}",
            self.endpoint,
            MANGA_COLLECTION_NAME,
            // filter
            urlencoding::encode(&filter)
        );
        debug!("get_manga: {}", url);

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

        if status.is_success() {
            let res = res
                .json::<Result>()
                .map_err(Error::FailedToParseResponseJson)?;

            if res.total_items > 1 {
                return Err(Error::MoreThenOneManga);
            }

            if res.items.len() <= 0 {
                return Err(Error::NoMangasWithName(name.to_string()));
            }

            Ok(res.items[0].clone())
        } else {
            debug!(
                "get_manga (REQUEST FAILED {}): {:?}",
                status,
                res.json::<serde_json::Value>().unwrap()
            );

            Err(Error::RequestFailed(status))
        }
    }

    pub fn create_manga<P>(
        &self,
        dir: P,
        metadata: &MangaMetadata,
    ) -> Result<Manga>
    where
        P: AsRef<Path>,
    {
        let url = format!(
            "{}/api/collections/{}/records",
            self.endpoint, MANGA_COLLECTION_NAME,
        );
        trace!("create_manga (URL): {}", url);

        create_form_from_metadata();

        let out = dir.as_ref().to_path_buf();

        let mut banner = out.clone();
        banner.push(&metadata.images.banner);

        let mut cover_medium = out.clone();
        cover_medium.push(&metadata.images.cover_medium);

        let mut cover_large = out.clone();
        cover_large.push(&metadata.images.cover_large);

        let mut cover_extra_large = out.clone();
        cover_extra_large.push(&metadata.images.cover_extra_large);

        let form = Form::new()
            .text("name", metadata.name.to_string())
            .text("englishTitle", metadata.english_title.to_string())
            .text("nativeTitle", metadata.native_title.to_string())
            .text("romajiTitle", metadata.romaji_title.to_string())
            .text("malUrl", metadata.mal_url.to_string())
            .text("anilistUrl", metadata.anilist_url.to_string())
            .text("description", metadata.description.to_string())
            .text("isGroup", metadata.is_group.to_string())
            .file("banner", banner)
            .map_err(Error::FailedToIncludeFileInForm)?
            .file("coverMedium", cover_medium)
            .map_err(Error::FailedToIncludeFileInForm)?
            .file("coverLarge", cover_large)
            .map_err(Error::FailedToIncludeFileInForm)?
            .file("coverExtraLarge", cover_extra_large)
            .map_err(Error::FailedToIncludeFileInForm)?
            ;

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
            debug!(
                "create_manga (REQUEST FAILED {}): {:?}",
                status,
                res.json::<serde_json::Value>().unwrap()
            );

            Err(Error::RequestFailed(status))
        }
    }

    fn get_chapter_page(
        &self,
        manga_id: &str,
        page: usize,
    ) -> Result<ChapterPage> {
        let filter = format!("(manga='{}')", manga_id);
        let url = format!(
            "{}/api/collections/{}/records?page={}&perPage=999&sort=idx&filter={}",
            self.endpoint,
            CHAPTERS_COLLECTION_NAME,
            page,
            urlencoding::encode(&filter)
        );
        trace!("get_chapter_page: {}", url);

        let res = self
            .client
            .get(url)
            .send()
            .map_err(Error::FailedToSendRequest)?;

        let status = res.status();

        if status.is_success() {
            let res = res
                .json::<ChapterPage>()
                .map_err(Error::FailedToParseResponseJson)?;

            Ok(res)
        } else {
            Err(Error::RequestFailed(status))
        }
    }

    pub fn get_chapters(&self, manga: &Manga) -> Result<Vec<Chapter>> {
        let mut res = Vec::new();

        let first_page = self.get_chapter_page(&manga.id, 1)?;
        res.reserve(first_page.total_items);

        res.extend_from_slice(&first_page.items);

        if first_page.total_pages > 0 {
            let num_pages = first_page.total_pages - 1;

            for page in 0..num_pages {
                let page = (first_page.page + 1) + page;

                let page = self.get_chapter_page(&manga.id, page)?;
                res.extend_from_slice(&page.items);
            }
        }

        Ok(res)
    }

    pub fn add_chapter(
        &self,
        manga: &Manga,
        index: usize,
        name: String,
        pages: &[PathBuf],
    ) -> Result<Chapter> {
        let url = format!(
            "{}/api/collections/{}/records",
            self.endpoint, CHAPTERS_COLLECTION_NAME,
        );
        trace!("add_chapter: {}", url);

        let cover = &pages[0];

        let mut form = Form::new()
            .text("idx", index.to_string())
            .text("name", name.to_string())
            .text("manga", manga.id.clone())
            .file("cover", cover)
            .unwrap();

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
            debug!(
                "add_chapter {} (REQUEST FAILED {}): {:?}",
                index,
                status,
                res.json::<serde_json::Value>().unwrap()
            );
            Err(Error::RequestFailed(status))
        }
    }
}
