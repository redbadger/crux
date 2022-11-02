use serde::{Deserialize, Serialize};

const CAT_LOADING_URL: &str = "https://c.tenor.com/qACzaJ1EBVYAAAAd/tenor.gif";

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct CatImage {
    pub file: String,
}

impl Default for CatImage {
    fn default() -> Self {
        Self {
            file: CAT_LOADING_URL.to_string(),
        }
    }
}

#[derive(Default)]
pub struct ViewModel {
    pub fact: String,
    pub image: Option<CatImage>,
    pub platform: String,
}

#[derive(Serialize, Deserialize)]
pub enum Msg {
    None,
    GetPlatform,
    SetPlatform { platform: String },
    Clear,
    Get,
    Fetch,
    Restore,                             // restore state
    SetState { bytes: Option<Vec<u8>> }, // receive the data to restore state with
    SetFact { bytes: Vec<u8> },
    SetImage { bytes: Vec<u8> },
    CurrentTime { iso_time: String },
}
