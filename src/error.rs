#[derive(Debug)]
pub enum Error {
    FailedToSendRequest(reqwest::Error),
    RequestFailed(reqwest::StatusCode),
    NoMangasWithName(String),
    MoreThenOneManga,

    FailedToIncludeFileInForm(std::io::Error),
    FailedToParseResponseJson(reqwest::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
