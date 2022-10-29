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
            .insert(uuid.clone(), Box::new(msg));

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
            .insert(uuid.clone(), Box::new(msg));

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
        F: Send + Sync + FnOnce(Vec<u8>) -> Msg,
    {
        self.http.get(url, msg)
    }

    pub fn time<F>(&self, msg: F) -> Request
    where
        F: Send + Sync + FnOnce(String) -> Msg,
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

trait App: Default {
    type Model: Default;
    type Msg;
    type ViewModel;

    fn update(&self, msg: Self::Msg, model: &mut Self::Model, cmd: &Cmd<Self::Msg>) -> Request;

    fn view(&self, model: &Self::Model) -> Self::ViewModel;
}

pub struct Core<A: App> {
    model: RwLock<<A as App>::Model>,
    cmd: Cmd<<A as App>::Msg>,
    app: A,
}

impl<A: App> Default for Core<A> {
    fn default() -> Self {
        Self {
            model: Default::default(),
            cmd: Default::default(),
            app: Default::default(),
        }
    }
}

impl<A: App> Core<A> {
    pub fn new() -> Self {
        Self::default()
    }

    // Direct message
    pub fn message(&self, msg: A::Msg) -> Request {
        let mut model = self.model.write().unwrap();
        let cmd = self.app.update(msg, &mut model, &self.cmd);

        cmd
    }

    // Return from capability
    pub fn response(&self, res: Response) -> Request {
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
}

// Application

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

#[derive(Default)]
struct CatFacts {}

#[derive(PartialEq, Default)]
struct Model {
    cat_fact: Option<CatFact>,
    time: Option<String>,
}

#[derive(Default)]
pub struct ViewModel {
    pub fact: String,
}

enum Msg {
    Clear,
    Get,
    Fetch,
    SetFact { bytes: Vec<u8> },
    SetImage { bytes: Vec<u8> },
    CurrentTime { iso_time: String },
}

impl App for CatFacts {
    type Model = Model;
    type Msg = Msg;
    type ViewModel = ViewModel;

    fn update(&self, msg: Msg, model: &mut Model, cmd: &Cmd<Msg>) -> Request {
        match msg {
            Msg::Clear => {
                model.cat_fact = None;

                cmd.render()
            }
            Msg::Get => {
                if let Some(_fact) = model.cat_fact {
                    cmd.render()
                } else {
                    cmd.http_get(API_URL.to_owned(), |bytes| Msg::SetFact { bytes })
                }
            }
            Msg::Fetch => cmd.http_get(API_URL.to_owned(), |bytes| Msg::SetFact { bytes }),

            Msg::SetFact { bytes } => {
                let fact = serde_json::from_slice::<CatFact>(&bytes).unwrap();
                model.cat_fact = Some(fact);

                // remember when we got the fact
                cmd.time(|iso_time| Msg::CurrentTime { iso_time })
            }
            Msg::CurrentTime { iso_time } => {
                model.time = Some(iso_time);

                cmd.render()
            }
            Msg::SetImage { bytes: _bytes } => cmd.render(),
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        let fact = match model {
            Model {
                cat_fact: Some(fact),
                time: Some(time),
            } => format!("Fact from {}: {}", time, fact.format()),
            _ => "No fact".to_string(),
        };

        ViewModel { fact }
    }
}
