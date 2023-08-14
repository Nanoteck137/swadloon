use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    FailedToSendRequest(reqwest::Error),
    RequestFailed(reqwest::StatusCode),
    NoMangasWithName(String),
    MoreThenOneManga,

    FailedToRetriveManga(Box<Error>),

    FailedToIncludeFileInForm(std::io::Error),
    FailedToParseResponseJson(reqwest::Error),

    PathNotDirectory(PathBuf),
    NoSeriesInfo(PathBuf),
    NoMangaSpec(PathBuf),
    NoCoverImage(PathBuf),

    InvalidMangaSpec(PathBuf),
    InvalidSeriesInfo(PathBuf),

    ReadMangaSpecUnknown(std::io::Error),

    FailedToGetLocalChapters,
}

pub type Result<T> = std::result::Result<T, Error>;
