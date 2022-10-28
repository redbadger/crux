use serde::Deserialize;
use std::{collections::HashMap, sync::RwLock};
use uuid::Uuid;

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
    Clear,
    Get,
    Fetch,
    SetFact { bytes: Vec<u8> },
    SetImage { bytes: Vec<u8> },
    CurrentTime { iso_time: String },
}

pub enum Cmd {
    HttpGet { url: String, uuid: Vec<u8> },
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
    http_continuations: RwLock<Continuations<Vec<u8>>>,
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
            Msg::Clear => {
                self.model.write().unwrap().cat_fact = None;

                Cmd::Render
            }
            Msg::Get => {
                if let Some(_fact) = &self.model.read().unwrap().cat_fact {
                    Cmd::Render
                } else {
                    Cmd::HttpGet {
                        url: API_URL.to_owned(),
                        uuid: self
                            .http_continuations
                            .write()
                            .unwrap()
                            .create(|bytes| Msg::SetFact { bytes }),
                    }
                }
            }
            Msg::Fetch => Cmd::HttpGet {
                url: API_URL.to_owned(),
                uuid: self
                    .http_continuations
                    .write()
                    .unwrap()
                    .create(|bytes| Msg::SetFact { bytes }),
            },
            Msg::SetFact { bytes } => {
                let fact = serde_json::from_slice::<CatFact>(&bytes).unwrap();
                self.model.write().unwrap().cat_fact = Some(fact);

                // remember when we got the fact
                Cmd::TimeGet
            }
            Msg::CurrentTime { iso_time } => {
                self.model.write().unwrap().time = Some(iso_time);

                Cmd::Render
            }
            Msg::SetImage { bytes: _bytes } => Cmd::Render,
        }
    }

    pub fn http_response(&self, uuid: Vec<u8>, bytes: Vec<u8>) -> Msg {
        self.http_continuations.write().unwrap().call(&uuid, bytes)
    }
}

#[derive(Default)]
struct Continuations<T> {
    table: RwLock<HashMap<[u8; 16], Box<dyn FnOnce(T) -> Msg + Sync + Send>>>,
}

impl<T> Continuations<T> {
    fn create<F>(&mut self, continuation: F) -> Vec<u8>
    where
        F: Sync + Send + FnOnce(T) -> Msg + 'static,
    {
        let uuid = *Uuid::new_v4().as_bytes();

        self.table
            .write()
            .unwrap()
            .insert(uuid.clone(), Box::new(continuation));

        uuid.to_vec()
    }

    pub fn call(&mut self, uuid: &[u8], data: T) -> Msg {
        let mut continuations = self.table.write().unwrap();
        let f = continuations.remove(uuid).unwrap();

        f(data)
    }
}
