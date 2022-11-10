mod continuations;
pub mod http;
pub mod key_value;
pub mod platform;
pub mod time;

use continuations::ContinuationStore;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

pub trait App: Default {
    type Message;
    type Model: Default;
    type ViewModel: Serialize;

    fn update(
        &self,
        msg: <Self as App>::Message,
        model: &mut <Self as App>::Model,
    ) -> Vec<Command<<Self as App>::Message>>;

    fn view(&self, model: &<Self as App>::Model) -> <Self as App>::ViewModel;
}

pub struct Command<Message> {
    body: RequestBody,
    msg_constructor: Option<Box<dyn FnOnce(ResponseBody) -> Message + Send + Sync + 'static>>,
}

impl<Message> Command<Message> {
    pub fn render() -> Command<Message> {
        Command {
            body: RequestBody::Render,
            msg_constructor: None,
        }
    }
}

pub struct AppCore<A: App> {
    model: RwLock<A::Model>,
    continuations: ContinuationStore<A::Message>,
    app: A,
}

impl<A: App> PartialEq for AppCore<A> {
    fn eq(&self, _other: &Self) -> bool {
        false // Core has all kinds of interior mutability
    }
}

impl<A: App> Default for AppCore<A> {
    fn default() -> Self {
        Self {
            model: Default::default(),
            continuations: Default::default(),
            app: Default::default(),
        }
    }
}

impl<A: App> AppCore<A> {
    pub fn new() -> Self {
        Self::default()
    }

    // Direct message
    pub fn message<'de, F>(&self, msg: &'de [u8]) -> Vec<u8>
    where
        <A as App>::Message: Deserialize<'de> + 'static,
    {
        let msg: <A as App>::Message = bcs::from_bytes(msg).unwrap();

        let mut model = self.model.write().unwrap();

        let commands: Vec<Command<<A as App>::Message>> = self.app.update(msg, &mut model);
        let requests: Vec<Request> = commands
            .into_iter()
            .map(|c| self.continuations.pause(c))
            .collect();

        bcs::to_bytes(&requests).unwrap()
    }

    // Return from capability
    pub fn response<'de, F>(&self, res: &'de [u8]) -> Vec<u8>
    where
        <A as App>::Message: Deserialize<'de> + 'static,
        F: (FnOnce(ResponseBody) -> <A as App>::Message) + Send + Sync + 'static,
    {
        let response = bcs::from_bytes(res).unwrap();
        let msg = self.continuations.resume(response);

        let mut model = self.model.write().unwrap();

        let commands: Vec<Command<<A as App>::Message>> = self.app.update(msg, &mut model);
        let requests: Vec<Request> = commands
            .into_iter()
            .map(|c| self.continuations.pause(c))
            .collect();

        bcs::to_bytes(&requests).unwrap()
    }

    pub fn view(&self) -> Vec<u8> {
        let model = self.model.read().unwrap();

        let value = self.app.view(&model);
        bcs::to_bytes(&value).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub uuid: Vec<u8>,
    pub body: RequestBody,
}

impl Request {
    pub fn render() -> Self {
        Self {
            uuid: Default::default(),
            body: RequestBody::Render,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RequestBody {
    Time,
    Http(String),
    Platform,
    KVRead(String),
    KVWrite(String, Vec<u8>),
    Render,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Response {
    pub uuid: Vec<u8>,
    pub body: ResponseBody,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum ResponseBody {
    Http(Vec<u8>),
    Time(String),
    Platform(String),
    KVRead(Option<Vec<u8>>),
    KVWrite(bool),
}
