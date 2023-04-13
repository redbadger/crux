#[derive(Debug)]
pub enum Error {
    Tauri(tauri::Error),
    Http(surf::Error),
    HttpConfig(String),
    HttpResponse(u16, String),
    HttpDecode(String),
    Url(surf::http::url::ParseError),
}

impl From<tauri::Error> for self::Error {
    fn from(error: tauri::Error) -> Self {
        Self::Tauri(error)
    }
}

impl From<surf::Error> for self::Error {
    fn from(error: surf::Error) -> Self {
        Self::Http(error)
    }
}

impl From<surf::http::url::ParseError> for self::Error {
    fn from(error: surf::http::url::ParseError) -> Self {
        Self::Url(error)
    }
}
