use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Debug)]
pub struct MetadataCoverImage {
    pub color: String,
    pub medium: String,
    pub large: String,
    #[serde(rename = "extraLarge")]
    pub extra_large: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MetadataDate {
    pub day: Option<usize>,
    pub month: Option<usize>,
    pub year: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MetadataTitle {
    pub english: Option<String>,
    pub native: String,
    pub romaji: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Metadata {
    pub id: usize,
    #[serde(rename = "idMal")]
    pub mal_id: Option<usize>,
    pub title: MetadataTitle,
    pub status: String,

    #[serde(rename = "type")]
    pub typ: String,
    pub format: String,

    pub description: String,
    pub genres: Vec<String>,

    pub chapters: Option<usize>,
    pub volumes: Option<usize>,

    #[serde(rename = "bannerImage")]
    pub banner_image: Option<String>,
    #[serde(rename = "coverImage")]
    pub cover_image: MetadataCoverImage,

    #[serde(rename = "startDate")]
    pub start_date: MetadataDate,
    #[serde(rename = "endDate")]
    pub end_date: MetadataDate,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SearchResult {
    pub id: usize,
    #[serde(rename = "idMal")]
    pub mal_id: Option<usize>,
    pub title: MetadataTitle,
}

// NOTE(patrik): https://anilist.co/graphiql
const MANGA_QUERY: &str = "
query ($id: Int) {
  Media(idMal: $id) {
    id
    idMal
    description(asHtml: true)
    type
    format
    status(version: 2)
    genres
    title {
      romaji
      english
      native
    }
    volumes
    chapters
    coverImage {
      medium
      extraLarge
      large
      color
    }
    bannerImage
    startDate {
      year
      month
      day
    }
    endDate {
      year
      month
      day
    }
  }
}
";

const SEARCH_QUERY: &str = "
query ($query: String) {
  Page(page: 1, perPage: 15) {
    media(search: $query, type: MANGA) {
      id
      idMal
      title {
        romaji
        english
        native
      }
    }
  }
}
";

pub fn fetch_anilist_metadata(mal_id: usize) -> Metadata {
    let client = Client::new();

    let json = json!({
        "query": MANGA_QUERY,
        "variables": {
            "id": mal_id
        }
    });

    let res = client
        .post("https://graphql.anilist.co/")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(json.to_string())
        .send()
        .unwrap();

    let headers = res.headers();
    println!("Headers: {:#?}", headers);

    // let date = headers.get("date").unwrap();
    // let date = date.to_str().unwrap();
    // let date = DateTime::parse_from_rfc2822(date).unwrap();
    // println!("Date: {:?}", date);

    let limit = headers.get("x-ratelimit-limit").unwrap();
    let limit = limit.to_str().unwrap();
    let limit = limit.parse::<usize>().unwrap();
    println!("Limit: {}", limit);

    let remaining = headers.get("x-ratelimit-remaining").unwrap();
    let remaining = remaining.to_str().unwrap();
    let remaining = remaining.parse::<usize>().unwrap();
    println!("Remaining: {}", remaining);

    if !res.status().is_success() {
        panic!("Request Error");
    }

    let j = res.json::<serde_json::Value>().unwrap();

    let media = j.get("data").unwrap().get("Media").unwrap();
    println!("Media: {:#?}", media);
    let res = serde_json::from_value::<Metadata>(media.clone()).unwrap();

    res
}

pub fn query(query: &str) -> Vec<SearchResult> {
    let client = Client::new();

    let json = json!({
        "query": SEARCH_QUERY,
        "variables": {
            "query": query
        }
    });

    let res = client
        .post("https://graphql.anilist.co/")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(json.to_string())
        .send()
        .unwrap();

    let headers = res.headers();
    println!("Headers: {:#?}", headers);

    // let date = headers.get("date").unwrap();
    // let date = date.to_str().unwrap();
    // let date = DateTime::parse_from_rfc2822(date).unwrap();
    // println!("Date: {:?}", date);

    let limit = headers.get("x-ratelimit-limit").unwrap();
    let limit = limit.to_str().unwrap();
    let limit = limit.parse::<usize>().unwrap();
    println!("Limit: {}", limit);

    let remaining = headers.get("x-ratelimit-remaining").unwrap();
    let remaining = remaining.to_str().unwrap();
    let remaining = remaining.parse::<usize>().unwrap();
    println!("Remaining: {}", remaining);

    if !res.status().is_success() {
        panic!("Request Error");
    }

    let j = res.json::<serde_json::Value>().unwrap();

    let results = j
        .get("data")
        .unwrap()
        .get("Page")
        .unwrap()
        .get("media")
        .unwrap();

    serde_json::from_value::<Vec<SearchResult>>(results.clone()).unwrap()
}
