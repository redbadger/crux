use serde::Deserialize;
use std::sync::RwLock;

const API_URL: &str = "https://catfact.ninja/fact";

#[derive(Deserialize, Default, Clone)]
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
    GetNewFact,
    ReceiveFact { bytes: Vec<u8> },
}

pub enum Effect {
    Render { cat_fact: String },
    Get { url: String },
}

#[derive(Default)]
pub struct Core {
    cat_fact: RwLock<CatFact>,
}

impl Core {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&self, msg: Msg) -> Effect {
        match msg {
            Msg::GetNewFact => Effect::Get {
                url: API_URL.to_owned(),
            },
            Msg::ReceiveFact { bytes } => {
                let cat_fact = serde_json::from_slice::<CatFact>(&bytes).unwrap();
                *self.cat_fact.write().unwrap() = cat_fact.clone();
                Effect::Render {
                    cat_fact: cat_fact.format(),
                }
            }
        }
    }
}
