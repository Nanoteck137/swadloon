use std::path::PathBuf;

use log::{debug, error, trace};
use reqwest::blocking::{multipart::Form, Client};
use reqwest::blocking::{ClientBuilder, Response};
use serde::{Deserialize, Serialize};

use crate::{ChapterEntry, ResolvedImages};

use crate::anilist::Metadata;
use crate::error::{Error, Result};

const MANGA_COLLECTION_NAME: &str = "mangas";
const CHAPTERS_COLLECTION_NAME: &str = "chapters";

#[derive(Clone, Debug)]
pub struct MangaMetadata {
    pub title: String,
    pub mal_id: usize,
    pub anilist_id: usize,
    pub description: String,

    pub color: String,
    pub banner: Option<PathBuf>,
    pub cover: PathBuf,

    pub start_date: String,
    pub end_date: String,
}

impl From<(Metadata, ResolvedImages)> for MangaMetadata {
    fn from(value: (Metadata, ResolvedImages)) -> Self {
        let (metadata, images) = value;
        MangaMetadata {
            title: metadata.title.english.unwrap_or(metadata.title.romaji),
            mal_id: metadata.mal_id.unwrap_or(0),
            anilist_id: metadata.id,
            description: metadata.description,
            color: metadata.cover_image.color.unwrap_or("".to_string()),
            banner: images.banner,
            cover: images.cover_extra_large,
            // TODO(patrik): Add date
            start_date: "".to_string(),
            end_date: "".to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChapterMetadata {
    index: usize,
    name: String,
    cover: PathBuf,
    pages: Vec<PathBuf>,
}

impl ChapterMetadata {
    pub fn new(
        index: usize,
        name: String,
        cover: PathBuf,
        pages: Vec<PathBuf>,
    ) -> Self {
        Self {
            index,
            name,
            cover,
            pages,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Manga {
    pub id: String,

    pub title: String,

    #[serde(rename = "malId")]
    pub mal_id: usize,
    #[serde(rename = "anilistId")]
    pub anilist_id: usize,

    pub description: String,

    pub color: String,
    pub banner: String,
    pub cover: String,

    #[serde(rename = "startDate")]
    pub start_date: String,
    #[serde(rename = "endDate")]
    pub end_date: String,

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

#[derive(Clone, Debug)]
pub struct Server {
    endpoint: String,
    client: Client,
}

fn print_status_error(prefix: &str, res: Response) {
    let status = res.status();
    if status == 400 {
        if let Ok(j) = res.json::<serde_json::Value>() {
            error!("{} [400 BAD REQUEST]: {:?}", prefix, j);
        } else {
            error!("{} [400 BAD REQUEST]", prefix);
        }
    } else {
        error!("{} [{} UNKNOWN ERROR]", prefix, status);
    }
}

#[derive(Deserialize, Debug)]
struct MangaPage {
    items: Vec<Manga>,
    page: usize,
    #[serde(rename = "perPage")]
    per_page: usize,
    #[serde(rename = "totalItems")]
    total_items: usize,
    #[serde(rename = "totalPages")]
    total_pages: usize,
}

impl Server {
    pub fn new(endpoint: String) -> Self {
        let client = ClientBuilder::new().timeout(None).build().unwrap();

        Self { endpoint, client }
    }

    fn get_manga_page(&self, page: usize) -> Result<MangaPage> {
        debug!("get_manga_page");

        let url = format!(
            "{}/api/collections/{}/records?page={}",
            self.endpoint, MANGA_COLLECTION_NAME, page
        );
        trace!("URL: {}", url);

        let res = self
            .client
            .get(url)
            .send()
            .map_err(Error::ServerSendRequestFailed)?;

        if res.status().is_success() {
            let manga = res.json::<MangaPage>().unwrap();
            return Ok(manga);
        } else {
            print_status_error("get_manga_page", res);
            Err(Error::ServerRequestFailed)
        }
    }

    pub fn get_all_manga(&self) -> Result<Vec<Manga>> {
        let mut res = Vec::new();

        let first_page = self.get_manga_page(1)?;
        res.reserve(first_page.total_items);

        res.extend_from_slice(&first_page.items);

        if first_page.total_pages > 0 {
            let num_pages = first_page.total_pages - 1;

            for page in 0..num_pages {
                let page = (first_page.page + 1) + page;

                let page = self.get_manga_page(page)?;
                res.extend_from_slice(&page.items);
            }
        }

        Ok(res)
    }

    pub fn get_manga(&self, anilist_id: usize) -> Result<Manga> {
        debug!("get_manga('{}')", anilist_id);

        let filter = format!("(anilistId='{}')", anilist_id);

        let url = format!(
            "{}/api/collections/{}/records?filter={}",
            self.endpoint,
            MANGA_COLLECTION_NAME,
            // filter
            urlencoding::encode(&filter)
        );
        trace!("URL: {}", url);

        #[derive(Deserialize)]
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
            .map_err(Error::ServerSendRequestFailed)?;

        let status = res.status();

        if status.is_success() {
            let res = res
                .json::<Result>()
                .map_err(Error::ServerResponseParseFailed)?;

            if res.total_items > 1 {
                error!("Expected one manga got: {}", res.total_items);
                return Err(Error::ServerWrongItemCount);
            }

            if res.items.len() <= 0 {
                return Err(Error::ServerNoRecord);
            }

            Ok(res.items[0].clone())
        } else {
            print_status_error("get_manga", res);
            Err(Error::ServerRequestFailed)
        }
    }

    pub fn update_manga(
        &self,
        manga: &Manga,
        metadata: MangaMetadata,
    ) -> Result<Manga> {
        let url = format!(
            "{}/api/collections/{}/records/{}",
            self.endpoint, MANGA_COLLECTION_NAME, manga.id,
        );
        debug!("update_manga (URL): {}", url);

        // FIXME(patrik): Use dates from metadata
        let start_date = "2020-04-02";
        let end_date = "2020-04-02";

        // TODO(patrik): Should we update malId and anilistId?
        let mut form = Form::new()
            .text("title", metadata.title.to_string())
            .text("anilistId", metadata.anilist_id.to_string())
            .text("malId", metadata.mal_id.to_string())
            .text("description", metadata.description.to_string())
            .text("startDate", start_date)
            .text("endDate", end_date)
            .text("color", metadata.color)
            .file("cover", &metadata.cover)
            .map_err(|e| {
                error!("Failed to include 'cover' in form");
                Error::ServerFormFileFailed(e)
            })?;

        if let Some(banner) = &metadata.banner {
            form = form.file("banner", &banner).map_err(|e| {
                error!("Failed to include 'banner' in form");
                Error::ServerFormFileFailed(e)
            })?;
        }

        let res = self
            .client
            .patch(url)
            .multipart(form)
            .send()
            .map_err(Error::ServerSendRequestFailed)?;

        let status = res.status();

        if status.is_success() {
            let manga = res
                .json::<Manga>()
                .map_err(Error::ServerResponseParseFailed)?;

            Ok(manga)
        } else {
            print_status_error("update_manga", res);
            Err(Error::ServerRequestFailed)
        }
    }

    pub fn create_manga(&self, metadata: MangaMetadata) -> Result<Manga> {
        let url = format!(
            "{}/api/collections/{}/records",
            self.endpoint, MANGA_COLLECTION_NAME,
        );
        trace!("create_manga (URL): {}", url);

        // FIXME(patrik): Use dates from metadata
        let start_date = "2020-04-02";
        let end_date = "2020-04-02";

        let mut form = Form::new()
            .text("title", metadata.title.to_string())
            .text("malId", metadata.mal_id.to_string())
            .text("anilistId", metadata.anilist_id.to_string())
            .text("description", metadata.description)
            .text("startDate", start_date)
            .text("endDate", end_date)
            .text("color", metadata.color)
            .file("cover", &metadata.cover)
            .map_err(|e| {
                error!("Failed to include 'cover' in form");
                Error::ServerFormFileFailed(e)
            })?;

        if let Some(banner) = &metadata.banner {
            form = form.file("banner", &banner).map_err(|e| {
                error!("Failed to include 'banner' in form");
                Error::ServerFormFileFailed(e)
            })?;
        }

        let res = self
            .client
            .post(url)
            .multipart(form)
            .send()
            .map_err(Error::ServerSendRequestFailed)?;

        let status = res.status();

        if status.is_success() {
            let manga = res
                .json::<Manga>()
                .map_err(Error::ServerResponseParseFailed)?;

            Ok(manga)
        } else {
            print_status_error("create_manga", res);
            Err(Error::ServerRequestFailed)
        }
    }

    pub fn delete_manga(&self, id: &str) -> Result<()> {
        let url = format!(
            "{}/api/collections/{}/records/{}",
            self.endpoint, MANGA_COLLECTION_NAME, id
        );

        let res = self
            .client
            .delete(url)
            .send()
            .map_err(Error::ServerSendRequestFailed)?;

        if res.status().is_success() {
            Ok(())
        } else {
            print_status_error("delete_manga", res);
            Err(Error::ServerRequestFailed)
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
            .map_err(Error::ServerSendRequestFailed)?;

        let status = res.status();

        if status.is_success() {
            let res = res
                .json::<ChapterPage>()
                .map_err(Error::ServerResponseParseFailed)?;

            Ok(res)
        } else {
            print_status_error("get_chapter_page", res);
            Err(Error::ServerRequestFailed)
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
        metadata: ChapterMetadata,
    ) -> Result<Chapter> {
        let url = format!(
            "{}/api/collections/{}/records",
            self.endpoint, CHAPTERS_COLLECTION_NAME,
        );
        trace!("add_chapter: {}", url);

        let mut form = Form::new()
            .text("idx", metadata.index.to_string())
            .text("name", metadata.name)
            .text("manga", manga.id.clone())
            .file("cover", metadata.cover)
            .map_err(|e| {
                error!("Failed to include 'cover' in form");
                Error::ServerFormFileFailed(e)
            })?;

        for page in metadata.pages {
            form = form.file("pages", page).map_err(|e| {
                error!("Failed to include 'page' in form");
                Error::ServerFormFileFailed(e)
            })?;
        }

        let res = self
            .client
            .post(url)
            .multipart(form)
            .send()
            .map_err(Error::ServerSendRequestFailed)?;

        let status = res.status();

        if status.is_success() {
            let res = res
                .json::<Chapter>()
                .map_err(Error::ServerResponseParseFailed)?;
            Ok(res)
        } else {
            print_status_error("add_chapter", res);
            Err(Error::ServerRequestFailed)
        }
    }

    pub fn update_chapter(
        &self,
        chapter: &Chapter,
        metadata: &ChapterEntry,
        cover: PathBuf,
        pages: Option<&[PathBuf]>,
    ) -> Result<Chapter> {
        let url = format!(
            "{}/api/collections/{}/records/{}",
            self.endpoint, CHAPTERS_COLLECTION_NAME, chapter.id
        );
        trace!("update_chapter: {}", url);

        // Clear out old pages so we can update with new ones
        if pages.is_some() {
            let value = serde_json::json!({
                "pages": null
            });
            // TODO(patrik): Check for errors
            self.client.patch(&url).json(&value).send().unwrap();
        }

        let mut form = Form::new()
            .text("name", metadata.name.to_string())
            .file("cover", cover)
            .map_err(|e| {
                error!("Failed to include 'cover' in form");
                Error::ServerFormFileFailed(e)
            })?;

        if let Some(pages) = pages {
            for (index, page) in pages.iter().enumerate() {
                form = form.file("pages", page).map_err(|e| {
                    error!("Failed to include page '{}' in form", index);
                    Error::ServerFormFileFailed(e)
                })?;
            }
        }

        let res = self
            .client
            .patch(url)
            .multipart(form)
            .send()
            .map_err(Error::ServerSendRequestFailed)?;

        let status = res.status();

        if status.is_success() {
            let res = res
                .json::<Chapter>()
                .map_err(Error::ServerResponseParseFailed)?;

            Ok(res)
        } else {
            print_status_error("update_chapter", res);
            Err(Error::ServerRequestFailed)
        }
    }
}
