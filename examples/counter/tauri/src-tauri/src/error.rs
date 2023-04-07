#[derive(Debug)]
pub enum Error {
    TauriError(tauri::Error),
    HttpError(surf::Error),
    HttpConfigError(String),
    HttpResponseError(u16, String),
    HttpDecodeError(String),
    UrlError(surf::http::url::ParseError),
}

impl From<tauri::Error> for self::Error {
    fn from(error: tauri::Error) -> Self {
        Self::TauriError(error)
    }
}

impl From<surf::Error> for self::Error {
    fn from(error: surf::Error) -> Self {
        Self::HttpError(error)
    }
}

impl From<surf::http::url::ParseError> for self::Error {
    fn from(error: surf::http::url::ParseError) -> Self {
        Self::UrlError(error)
    }
}
