use serde::{Deserialize, Serialize};
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

struct KeyValue<Msg> {
    reads: RwLock<HashMap<[u8; 16], Box<dyn FnOnce(Option<Vec<u8>>) -> Msg + Sync + Send>>>,
    writes: RwLock<HashMap<[u8; 16], Box<dyn FnOnce(bool) -> Msg + Sync + Send>>>,
}

impl<Msg> Default for KeyValue<Msg> {
    fn default() -> Self {
        Self {
            reads: RwLock::new(HashMap::new()),
            writes: RwLock::new(HashMap::new()),
        }
    }
}

impl<Msg> KeyValue<Msg> {
    pub fn write<F>(&self, key: String, bytes: Vec<u8>, msg: F) -> Request
    where
        F: Sync + Send + FnOnce(bool) -> Msg + 'static,
    {
        let uuid = *Uuid::new_v4().as_bytes();

        self.writes.write().unwrap().insert(uuid, Box::new(msg));

        Request::KVWrite {
            uuid: uuid.to_vec(),
            key,
            bytes,
        }
    }

    pub fn written(&self, uuid: &[u8], result: bool) -> Msg {
        let mut writes = self.writes.write().unwrap();
        let f = writes.remove(uuid).unwrap();

        f(result)
    }

    pub fn read<F>(&self, key: String, msg: F) -> Request
    where
        F: Sync + Send + FnOnce(Option<Vec<u8>>) -> Msg + 'static,
    {
        let uuid = *Uuid::new_v4().as_bytes();

        self.reads.write().unwrap().insert(uuid, Box::new(msg));

        Request::KVRead {
            uuid: uuid.to_vec(),
            key,
        }
    }

    pub fn receive_read(&self, uuid: &[u8], bytes: Option<Vec<u8>>) -> Msg {
        let mut reads = self.reads.write().unwrap();
        let f = reads.remove(uuid).unwrap();

        f(bytes)
    }
}

struct Cmd<Msg> {
    http: Http<Msg>,
    time: Time<Msg>,
    key_value: KeyValue<Msg>,
}

impl<Msg> Default for Cmd<Msg> {
    fn default() -> Self {
        Self {
            http: Http::default(),
            time: Time::default(),
            key_value: KeyValue::default(),
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

    pub fn kv_write<F>(&self, key: String, bytes: Vec<u8>, msg: F) -> Request
    where
        F: Send + Sync + 'static + FnOnce(bool) -> Msg,
    {
        self.key_value.write(key, bytes, msg)
    }

    pub fn kv_read<F>(&self, key: String, msg: F) -> Request
    where
        F: Send + Sync + 'static + FnOnce(Option<Vec<u8>>) -> Msg,
    {
        self.key_value.read(key, msg)
    }

    pub fn render(&self) -> Request {
        Request::Render
    }
}

pub enum Request {
    Http {
        uuid: Vec<u8>,
        url: String,
    },
    Time {
        uuid: Vec<u8>,
    },
    KVRead {
        uuid: Vec<u8>,
        key: String,
    },
    KVWrite {
        uuid: Vec<u8>,
        key: String,
        bytes: Vec<u8>,
    },
    Render,
}

pub enum Response {
    Http {
        uuid: Vec<u8>,
        bytes: Vec<u8>,
    },
    Time {
        uuid: Vec<u8>,
        iso_time: String,
    },
    KVRead {
        uuid: Vec<u8>,
        bytes: Option<Vec<u8>>,
    },
    KVWrite {
        uuid: Vec<u8>,
        success: bool,
    },
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
            Response::KVRead { uuid, bytes } => {
                let msg = self.cmd.key_value.receive_read(&uuid, bytes);

                self.app.update(msg, &mut model, &self.cmd)
            }
            Response::KVWrite { uuid, success } => {
                let msg = self.cmd.key_value.written(&uuid, success);

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

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct CatFact {
    fact: String,
    length: i32,
}

impl CatFact {
    fn format(&self) -> String {
        format!("{} ({} bytes)", self.fact, self.length)
    }
}

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
struct CatFacts {}

#[derive(Serialize, Deserialize, PartialEq, Default)]
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
    None,
    Clear,
    Get,
    Fetch,
    Restore,                             // restore state
    SetState { bytes: Option<Vec<u8>> }, // receive the data to restore state with
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
                let bytes = serde_json::to_vec(&model).unwrap();

                vec![
                    cmd.kv_write("state".to_string(), bytes, |_| Msg::None),
                    cmd.render(),
                ]
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

                let bytes = serde_json::to_vec(&model).unwrap();

                vec![
                    cmd.kv_write("state".to_string(), bytes, |_| Msg::None),
                    cmd.time(|iso_time| Msg::CurrentTime { iso_time }),
                ]
            }
            Msg::CurrentTime { iso_time } => {
                model.time = Some(iso_time);
                let bytes = serde_json::to_vec(&model).unwrap();

                vec![
                    cmd.kv_write("state".to_string(), bytes, |_| Msg::None),
                    cmd.render(),
                ]
            }
            Msg::SetImage { bytes } => {
                let image = serde_json::from_slice::<CatImage>(&bytes).unwrap();
                model.cat_image = Some(image);

                let bytes = serde_json::to_vec(&model).unwrap();

                vec![
                    cmd.kv_write("state".to_string(), bytes, |_| Msg::None),
                    cmd.render(),
                ]
            }
            Msg::Restore => {
                vec![cmd.kv_read("state".to_string(), |bytes| Msg::SetState { bytes })]
            }
            Msg::SetState { bytes } => {
                if let Some(bytes) = bytes {
                    *model = serde_json::from_slice::<Model>(&bytes).unwrap();
                }

                vec![cmd.render()]
            }
            Msg::None => vec![],
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
