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
    ReceiveFact { bytes: Vec<u8> },
}

pub enum Cmd {
    Render { cat_fact: String },
    Get { url: String },
}

#[derive(Default)]
pub struct Core {
    cat_fact: RwLock<Option<CatFact>>,
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

    pub fn update(&self, msg: Msg) -> Cmd {
        match msg {
            Msg::ClearFact => {
                *self.cat_fact.write().unwrap() = None;
                Cmd::Render {
                    cat_fact: "Cleared".to_string(),
                }
            }
            Msg::GetFact => {
                if let Some(fact) = &*self.cat_fact.read().unwrap() {
                    Cmd::Render {
                        cat_fact: format!("old: {}", fact.format()),
                    }
                } else {
                    Cmd::Get {
                        url: API_URL.to_owned(),
                    }
                }
            }
            Msg::FetchFact => Cmd::Get {
                url: API_URL.to_owned(),
            },
            Msg::ReceiveFact { bytes } => {
                let fact = serde_json::from_slice::<CatFact>(&bytes).unwrap();
                *self.cat_fact.write().unwrap() = Some(fact.clone());
                Cmd::Render {
                    cat_fact: format!("new: {}", fact.format()),
                }
            }
        }
    }
}
