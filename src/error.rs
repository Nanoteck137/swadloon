use reqwest::StatusCode;

#[derive(Debug)]
pub enum ServerKind {
    GetManga,
    UpdateManga,
    CreateManga,
    GetChapterPage,
    AddChapter,
    UpdateChapter,
}

#[derive(Debug)]
pub enum Error {
    FailedToSendRequest(reqwest::Error),
    RequestFailed(reqwest::StatusCode),
    NoMangasWithName(String),
    MoreThenOneManga,

    FailedToRetriveManga(Box<Error>),

    FailedToIncludeFileInForm(std::io::Error),
    FailedToParseResponseJson(reqwest::Error),

    ServerSendRequestFailed(ServerKind, reqwest::Error),
    ServerResponseParseFailed(ServerKind, reqwest::Error),
    ServerWrongItemCount(ServerKind), 
    ServerNoRecordWithName {
        kind: ServerKind,
        name: String,
    },
    ServerBadRequest {
        kind: ServerKind,
    },
    ServerRequestFailed(ServerKind, StatusCode), 
    ServerFormFileFailed(ServerKind, std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
