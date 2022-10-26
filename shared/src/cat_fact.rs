use serde::Deserialize;

const API_URL: &str = "https://catfact.ninja/fact";

#[derive(Deserialize, Default)]
struct CatFact {
    pub fact: String,
    pub length: i32,
}

impl CatFact {
    pub fn format(&self) -> String {
        format!("{} ({} bytes)", self.fact, self.length)
    }
}

#[derive(Default)]
pub struct State {
    pub cat_fact: String,
}

pub enum Msg {
    GetNewFact,
    ReceiveFact { bytes: Vec<u8> },
}

pub enum Effect {
    Render { state: State },
    Get { state: State, url: String },
}

pub fn update(state: State, msg: Msg) -> Effect {
    match msg {
        Msg::GetNewFact => Effect::Get {
            state,
            url: API_URL.to_owned(),
        },
        Msg::ReceiveFact { bytes } => {
            let cat_fact = serde_json::from_slice::<CatFact>(&bytes).unwrap();
            Effect::Render {
                state: State {
                    cat_fact: cat_fact.format(),
                },
            }
        }
    }
}
