//! TODO mod docs

/// TODO docs
#[cfg(feature = "typegen")]
pub mod typegen;

mod continuations;
pub mod http;
pub mod key_value;
pub mod platform;
pub mod time;

use continuations::ContinuationStore;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// TODO docs
pub trait App: Default {
    /// TODO docs
    type Message;
    /// TODO docs
    type Model: Default;
    /// TODO docs
    type ViewModel: Serialize;

    /// TODO docs
    fn update(
        &self,
        msg: <Self as App>::Message,
        model: &mut <Self as App>::Model,
    ) -> Vec<Command<<Self as App>::Message>>;

    /// TODO docs
    fn view(&self, model: &<Self as App>::Model) -> <Self as App>::ViewModel;
}

/// TODO docs
pub struct Command<Message> {
    body: RequestBody,
    msg_constructor: Option<Box<dyn FnOnce(ResponseBody) -> Message + Send + Sync>>,
}

impl<Message: 'static> Command<Message> {
    /// TODO docs
    pub fn render() -> Command<Message> {
        Command {
            body: RequestBody::Render,
            msg_constructor: None,
        }
    }

    /// TODO docs
    pub fn lift<ParentMsg, F>(commands: Vec<Command<Message>>, f: F) -> Vec<Command<ParentMsg>>
    where
        F: FnOnce(Message) -> ParentMsg + Sync + Send + Copy + 'static,
    {
        commands.into_iter().map(move |c| c.map(f)).collect()
    }

    fn map<ParentMsg, F>(self, f: F) -> Command<ParentMsg>
    where
        F: FnOnce(Message) -> ParentMsg + Sync + Send + 'static,
    {
        Command {
            body: self.body,
            msg_constructor: match self.msg_constructor {
                Some(g) => Some(Box::new(|b| f(g(b)))),
                None => None,
            },
        }
    }
}

/// TODO docs
pub struct Core<A: App> {
    model: RwLock<A::Model>,
    continuations: ContinuationStore<A::Message>,
    app: A,
}

impl<A: App> Default for Core<A> {
    fn default() -> Self {
        Self {
            model: Default::default(),
            continuations: Default::default(),
            app: Default::default(),
        }
    }
}

impl<A: App> Core<A> {
    /// TODO docs
    pub fn new() -> Self {
        Self::default()
    }

    /// TODO docs
    // Direct message
    pub fn message<'de>(&self, msg: &'de [u8]) -> Vec<u8>
    where
        <A as App>::Message: Deserialize<'de>,
    {
        let msg: <A as App>::Message =
            bcs::from_bytes(msg).expect("Message deserialization failed.");

        let mut model = self.model.write().expect("Model RwLock was poisoned.");

        let commands: Vec<Command<<A as App>::Message>> = self.app.update(msg, &mut model);
        let requests: Vec<Request> = commands
            .into_iter()
            .map(|c| self.continuations.pause(c))
            .collect();

        bcs::to_bytes(&requests).expect("Request serialization failed.")
    }

    /// TODO docs
    // Return from capability
    pub fn response<'de>(&self, res: &'de [u8]) -> Vec<u8>
    where
        <A as App>::Message: Deserialize<'de>,
    {
        let response = bcs::from_bytes(res).expect("Response deserialization failed.");
        let msg = self.continuations.resume(response);

        let mut model = self.model.write().expect("Model RwLock was poisoned.");

        let commands: Vec<Command<<A as App>::Message>> = self.app.update(msg, &mut model);
        let requests: Vec<Request> = commands
            .into_iter()
            .map(|c| self.continuations.pause(c))
            .collect();

        bcs::to_bytes(&requests).expect("Request serialization failed.")
    }

    /// TODO docs
    pub fn view(&self) -> Vec<u8> {
        let model = self.model.read().expect("Model RwLock was poisoned.");

        let value = self.app.view(&model);
        bcs::to_bytes(&value).expect("View model serialization failed.")
    }
}

/// TODO docs
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

/// TODO docs
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum RequestBody {
    Time,
    Http(String),
    Platform,
    KVRead(String),
    KVWrite(String, Vec<u8>),
    Render,
}

/// TODO docs
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Response {
    pub uuid: Vec<u8>,
    pub body: ResponseBody,
}

/// TODO docs
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum ResponseBody {
    Http(Vec<u8>),
    Time(String),
    Platform(String),
    KVRead(Option<Vec<u8>>),
    KVWrite(bool),
}
