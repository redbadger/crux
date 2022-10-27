use serde::Deserialize;
use std::sync::RwLock;

const API_URL: &str = "https://catfact.ninja/fact";

#[derive(Deserialize, Default, Clone, PartialEq)]
struct CatFact {
    fact: String,
    length: i32,
}

impl CatFact {
    fn format(&self) -> String {
        format!("{} ({} bytes)", self.fact, self.length)
    }
}

pub enum Msg {
    ClearFact,
    GetFact,
    FetchFact,
    HttpResponse { bytes: Vec<u8> },
    CurrentTime { iso_time: String },
}

pub enum Cmd {
    HttpGet { url: String },
    TimeGet,
    Render,
}

#[derive(Default)]
pub struct Core {
    cat_fact: RwLock<Option<CatFact>>,
    time: RwLock<Option<String>>,
}

impl PartialEq for Core {
    fn eq(&self, other: &Self) -> bool {
        let a = self.cat_fact.read().unwrap();
        let b = other.cat_fact.read().unwrap();
        *a == *b
    }
}

impl Core {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fact(&self) -> String {
        match (&*self.cat_fact.read().unwrap(), &*self.time.read().unwrap()) {
            (Some(fact), Some(time)) => format!("Fact from {}: {}", time, fact.format()),
            _ => "No fact".to_string(),
        }
    }

    pub fn update(&self, msg: Msg) -> Cmd {
        match msg {
            Msg::ClearFact => {
                *self.cat_fact.write().unwrap() = None;
                Cmd::Render
            }
            Msg::GetFact => {
                if let Some(_fact) = &*self.cat_fact.read().unwrap() {
                    Cmd::Render
                } else {
                    Cmd::HttpGet {
                        url: API_URL.to_owned(),
                    }
                }
            }
            Msg::FetchFact => Cmd::HttpGet {
                url: API_URL.to_owned(),
            },
            Msg::HttpResponse { bytes } => {
                let fact = serde_json::from_slice::<CatFact>(&bytes).unwrap();
                *self.cat_fact.write().unwrap() = Some(fact.clone());

                // remember when we got the fact
                Cmd::TimeGet
            }
            Msg::CurrentTime { iso_time } => {
                *self.time.write().unwrap() = Some(iso_time);

                Cmd::Render
            }
        }
    }
}
