use serde::Deserialize;
use std::{collections::HashMap, sync::RwLock};
use uuid::Uuid;

struct Http<Msg> {
    continuations: RwLock<HashMap<[u8; 16], Box<dyn FnOnce(Vec<u8>) -> Msg + Sync + Send>>>,
}

impl<Msg> Default for Http<Msg> {
    fn default() -> Self {
        Self {
            continuations: RwLock::new(HashMap::new()),
        }
    }
}

impl<Msg> Http<Msg> {
    pub fn get<F>(&self, url: String, msg: F) -> Request
    where
        F: Sync + Send + FnOnce(Vec<u8>) -> Msg + 'static,
    {
        let uuid = *Uuid::new_v4().as_bytes();

        self.continuations
            .write()
            .unwrap()
            .insert(uuid, Box::new(msg));

        Request::Http {
            uuid: uuid.to_vec(),
            url,
        }
    }

    pub fn receive(&self, uuid: &[u8], data: Vec<u8>) -> Msg {
        let mut continuations = self.continuations.write().unwrap();
        let f = continuations.remove(uuid).unwrap();

        f(data)
    }
}

struct Time<Msg> {
    continuations: RwLock<HashMap<[u8; 16], Box<dyn FnOnce(String) -> Msg + Sync + Send>>>,
}

impl<Msg> Default for Time<Msg> {
    fn default() -> Self {
        Self {
            continuations: RwLock::new(HashMap::new()),
        }
    }
}

impl<Msg> Time<Msg> {
    pub fn get<F>(&self, msg: F) -> Request
    where
        F: Sync + Send + FnOnce(String) -> Msg + 'static,
    {
        let uuid = *Uuid::new_v4().as_bytes();

        self.continuations
            .write()
            .unwrap()
            .insert(uuid, Box::new(msg));

        Request::Time {
            uuid: uuid.to_vec(),
        }
    }

    pub fn receive(&self, uuid: &[u8], data: String) -> Msg {
        let mut continuations = self.continuations.write().unwrap();
        let f = continuations.remove(uuid).unwrap();

        f(data)
    }
}

struct Cmd<Msg> {
    http: Http<Msg>,
    time: Time<Msg>,
}

impl<Msg> Default for Cmd<Msg> {
    fn default() -> Self {
        Self {
            http: Http::default(),
            time: Time::default(),
        }
    }
}

impl<Msg> Cmd<Msg> {
    pub fn http_get<F>(&self, url: String, msg: F) -> Request
    where
        F: Send + Sync + 'static + FnOnce(Vec<u8>) -> Msg,
    {
        self.http.get(url, msg)
    }

    pub fn time<F>(&self, msg: F) -> Request
    where
        F: Send + Sync + 'static + FnOnce(String) -> Msg,
    {
        self.time.get(msg)
    }

    pub fn render(&self) -> Request {
        Request::Render
    }
}

pub enum Request {
    Http { uuid: Vec<u8>, url: String },
    Time { uuid: Vec<u8> },
    Render,
}

pub enum Response {
    Http { uuid: Vec<u8>, bytes: Vec<u8> },
    Time { uuid: Vec<u8>, iso_time: String },
}

#[derive(Default)]
pub struct Core {
    model: RwLock<Model>,
    cmd: Cmd<Msg>,
    app: CatFacts,
}

impl PartialEq for Core {
    fn eq(&self, _other: &Self) -> bool {
        false // Core has all kinds of interior mutability
    }
}

impl Core {
    pub fn new() -> Self {
        Self::default()
    }

    // Direct message
    pub fn message(&self, msg: Msg) -> Vec<Request> {
        let mut model = self.model.write().unwrap();

        self.app.update(msg, &mut model, &self.cmd)
    }

    // Return from capability
    pub fn response(&self, res: Response) -> Vec<Request> {
        let mut model = self.model.write().unwrap();
        match res {
            Response::Http { uuid, bytes } => {
                let msg = self.cmd.http.receive(&uuid, bytes);

                self.app.update(msg, &mut model, &self.cmd)
            }
            Response::Time { uuid, iso_time } => {
                let msg = self.cmd.time.receive(&uuid, iso_time);

                self.app.update(msg, &mut model, &self.cmd)
            }
        }
    }

    pub fn view(&self) -> ViewModel {
        let model = self.model.read().unwrap();

        self.app.view(&model)
    }
}

// Application

const FACT_API_URL: &str = "https://catfact.ninja/fact";
const IMAGE_API_URL: &str = "https://aws.random.cat/meow";
const CAT_LOADING_URL: &str = "https://c.tenor.com/qACzaJ1EBVYAAAAd/tenor.gif";

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

#[derive(Deserialize, PartialEq, Eq, Clone)]
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
struct CatFacts {}

#[derive(PartialEq, Default)]
struct Model {
    cat_fact: Option<CatFact>,
    cat_image: Option<CatImage>,
    time: Option<String>,
}

#[derive(Default)]
pub struct ViewModel {
    pub fact: String,
    pub image: Option<CatImage>,
}

pub enum Msg {
    Clear,
    Get,
    Fetch,
    SetFact { bytes: Vec<u8> },
    SetImage { bytes: Vec<u8> },
    CurrentTime { iso_time: String },
}

impl CatFacts {
    fn update(&self, msg: Msg, model: &mut Model, cmd: &Cmd<Msg>) -> Vec<Request> {
        match msg {
            Msg::Clear => {
                model.cat_fact = None;
                model.cat_image = None;

                vec![cmd.render()]
            }
            Msg::Get => {
                if let Some(_fact) = &model.cat_fact {
                    vec![cmd.render()]
                } else {
                    model.cat_image = Some(CatImage::default());

                    vec![
                        cmd.http_get(FACT_API_URL.to_owned(), |bytes| Msg::SetFact { bytes }),
                        cmd.http_get(IMAGE_API_URL.to_string(), |bytes| Msg::SetImage { bytes }),
                        cmd.render(),
                    ]
                }
            }
            Msg::Fetch => {
                model.cat_image = Some(CatImage::default());

                vec![
                    cmd.http_get(FACT_API_URL.to_owned(), |bytes| Msg::SetFact { bytes }),
                    cmd.http_get(IMAGE_API_URL.to_string(), |bytes| Msg::SetImage { bytes }),
                    cmd.render(),
                ]
            }
            Msg::SetFact { bytes } => {
                let fact = serde_json::from_slice::<CatFact>(&bytes).unwrap();
                model.cat_fact = Some(fact);

                // remember when we got the fact
                vec![cmd.time(|iso_time| Msg::CurrentTime { iso_time })]
            }
            Msg::CurrentTime { iso_time } => {
                model.time = Some(iso_time);

                // TODO convert to parallel fetching
                vec![cmd.render()]
            }
            Msg::SetImage { bytes } => {
                let image = serde_json::from_slice::<CatImage>(&bytes).unwrap();

                model.cat_image = Some(image);

                vec![cmd.render()]
            }
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        let fact = match (&model.cat_fact, &model.time) {
            (Some(fact), Some(time)) => format!("Fact from {}: {}", time, fact.format()),
            _ => "No fact".to_string(),
        };

        ViewModel {
            fact,
            image: model.cat_image.clone(),
        }
    }
}
