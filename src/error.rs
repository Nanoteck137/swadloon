#[derive(Debug)]
pub enum Error {
    FailedToRetriveManga(Box<Error>),

    ServerSendRequestFailed(reqwest::Error),
    ServerResponseParseFailed(reqwest::Error),
    ServerWrongItemCount,
    ServerNoRecord,
    ServerRequestFailed,
    ServerFormFileFailed(std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
