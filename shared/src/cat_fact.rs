use serde::Deserialize;
use std::sync::RwLock;

const API_URL: &str = "https://catfact.ninja/fact";

#[derive(Deserialize, Default, Clone, PartialEq, Eq)]
pub struct CatFact {
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

#[derive(PartialEq, Default)]
struct Model {
    cat_fact: Option<CatFact>,
    time: Option<String>,
}

#[derive(Default)]
pub struct ViewModel {
    pub fact: String,
}

#[derive(Default)]
pub struct Core {
    model: RwLock<Model>,
}

impl PartialEq for Core {
    fn eq(&self, other: &Self) -> bool {
        let a = self.model.read().unwrap();
        let b = other.model.read().unwrap();
        *a == *b
    }
}

impl Core {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view(&self) -> ViewModel {
        let fact = match &*self.model.read().unwrap() {
            Model {
                cat_fact: Some(fact),
                time: Some(time),
            } => format!("Fact from {}: {}", time, fact.format()),
            _ => "No fact".to_string(),
        };

        ViewModel { fact }
    }

    pub fn update(&self, msg: Msg) -> Cmd {
        match msg {
            Msg::ClearFact => {
                self.model.write().unwrap().cat_fact = None;

                Cmd::Render
            }
            Msg::GetFact => {
                if let Some(_fact) = &self.model.read().unwrap().cat_fact {
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
                self.model.write().unwrap().cat_fact = Some(fact);

                // remember when we got the fact
                Cmd::TimeGet
            }
            Msg::CurrentTime { iso_time } => {
                self.model.write().unwrap().time = Some(iso_time);

                Cmd::Render
            }
        }
    }
}
