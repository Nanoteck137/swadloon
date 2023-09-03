use std::path::PathBuf;

use log::{debug, error, trace};
use reqwest::blocking::{multipart::Form, Client};
use reqwest::blocking::{ClientBuilder, Response};
use serde::{Deserialize, Serialize};
use swadloon::anilist::Metadata;
use swadloon::ChapterEntry;

use crate::error::{Error, Result};
use crate::shared::ResolvedImages;

const MANGA_COLLECTION_NAME: &str = "mangas";
const CHAPTERS_COLLECTION_NAME: &str = "chapters";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Manga {
    pub id: String,

    #[serde(rename = "malId")]
    pub mal_id: usize,
    #[serde(rename = "anilistId")]
    pub anilist_id: usize,

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

impl Server {
    pub fn new(endpoint: String) -> Self {
        let client = ClientBuilder::new().timeout(None).build().unwrap();

        Self { endpoint, client }
    }

    pub fn get_manga(&self, mal_id: usize) -> Result<Manga> {
        debug!("get_manga('{}')", mal_id);

        let filter = format!("(malId='{}')", mal_id);

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
        metadata: &Metadata,
        images: &ResolvedImages,
    ) -> Result<Manga> {
        let url = format!(
            "{}/api/collections/{}/records/{}",
            self.endpoint, MANGA_COLLECTION_NAME, manga.id,
        );
        debug!("update_manga (URL): {}", url);

        // TODO(patrik): Cleanup
        let mal_url = format!(
            "https://myanimelist.net/manga/{}",
            metadata.mal_id.unwrap()
        );
        let anilist_url = format!("https://anilist.co/manga/{}", metadata.id);

        // FIXME(patrik): Use dates from metadata
        let start_date = "2020-04-02";
        let end_date = "2020-04-02";

        // TODO(patrik): Should we update malId and anilistId?
        let mut form = Form::new()
            .text(
                "englishTitle",
                metadata
                    .title
                    .english
                    .as_ref()
                    .unwrap_or(&metadata.title.romaji)
                    .to_string(),
            )
            .text("nativeTitle", metadata.title.native.to_string())
            .text("romajiTitle", metadata.title.romaji.to_string())
            .text("malUrl", mal_url)
            .text("anilistUrl", anilist_url)
            .text("description", metadata.description.to_string())
            .text("startDate", start_date)
            .text("endDate", end_date)
            .text("color", metadata.cover_image.color.to_string())
            .file("coverMedium", &images.cover_medium)
            .map_err(|e| {
                error!("Failed to include 'coverMedium' in form");
                Error::ServerFormFileFailed(e)
            })?
            .file("coverLarge", &images.cover_large)
            .map_err(|e| {
                error!("Failed to include 'coverLarge' in form");
                Error::ServerFormFileFailed(e)
            })?
            .file("coverExtraLarge", &images.cover_extra_large)
            .map_err(|e| {
                error!("Failed to include 'coverExtraLarge' in form");
                Error::ServerFormFileFailed(e)
            })?;

        if let Some(banner) = &images.banner {
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

    pub fn create_manga(
        &self,
        metadata: &Metadata,
        images: &ResolvedImages,
    ) -> Result<Manga> {
        let url = format!(
            "{}/api/collections/{}/records",
            self.endpoint, MANGA_COLLECTION_NAME,
        );
        trace!("create_manga (URL): {}", url);

        // TODO(patrik): Cleanup
        let mal_url = format!(
            "https://myanimelist.net/manga/{}",
            metadata.mal_id.unwrap()
        );
        let anilist_url = format!("https://anilist.co/manga/{}", metadata.id);

        // FIXME(patrik): Use dates from metadata
        let start_date = "2020-04-02";
        let end_date = "2020-04-02";

        let mut form = Form::new()
            .text("malId", metadata.mal_id.unwrap().to_string())
            .text("anilistId", metadata.id.to_string())
            .text(
                "englishTitle",
                metadata
                    .title
                    .english
                    .as_ref()
                    .unwrap_or(&metadata.title.romaji)
                    .to_string(),
            )
            .text("nativeTitle", metadata.title.native.to_string())
            .text("romajiTitle", metadata.title.romaji.to_string())
            .text("malUrl", mal_url)
            .text("anilistUrl", anilist_url)
            .text("description", metadata.description.to_string())
            .text("startDate", start_date)
            .text("endDate", end_date)
            .text("color", metadata.cover_image.color.to_string())
            .file("coverMedium", &images.cover_medium)
            .map_err(|e| {
                error!("Failed to include 'coverMedium' in form");
                Error::ServerFormFileFailed(e)
            })?
            .file("coverLarge", &images.cover_large)
            .map_err(|e| {
                error!("Failed to include 'coverLarge' in form");
                Error::ServerFormFileFailed(e)
            })?
            .file("coverExtraLarge", &images.cover_extra_large)
            .map_err(|e| {
                error!("Failed to include 'coverExtraLarge' in form");
                Error::ServerFormFileFailed(e)
            })?;

        if let Some(banner) = &images.banner {
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
        metadata: &ChapterEntry,
        cover: PathBuf,
        pages: &[PathBuf],
    ) -> Result<Chapter> {
        let url = format!(
            "{}/api/collections/{}/records",
            self.endpoint, CHAPTERS_COLLECTION_NAME,
        );
        trace!("add_chapter: {}", url);

        let mut form = Form::new()
            .text("idx", metadata.index.to_string())
            .text("name", metadata.name.to_string())
            .text("manga", manga.id.clone())
            .file("cover", cover)
            .map_err(|e| {
                error!("Failed to include 'cover' in form");
                Error::ServerFormFileFailed(e)
            })?;

        for page in pages {
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
