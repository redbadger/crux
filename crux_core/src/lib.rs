//! Cross-platform app development in Rust
//!
//! Crux helps you share your app's business logic and behavior across mobile (iOS and Android) and web,
//! as a single, reusable core built with Rust.
//!
//! Unlike React Native, the user interface layer is built natively, with modern declarative UI frameworks
//! such as Swift UI, Jetpack Compose and React/Vue or a WASM based framework on the web.
//!
//! The UI layer is as thin as it can be, and all other work is done by the shared core.
//! The interface with the core has static type checking across languages.
//!
//! ## Getting Started
//!
//! Crux applications are split into two parts: Core written in Rust and a Shell written in the platform
//! native language (e.g. Swift or Kotlin).
//!  
//! Below is a minimal example of a Crux based application Core:
//!
//! ```rust
//! // src/app.rs
//!
//! use serde::{Serialize, Deserialize}
//! use crux_core::App
//!
//! // Model describing the application state
//! #[derive(Default)]
//! struct Model {
//!     count: isize,
//! }
//!
//! // Message describing the actions that can be taken
//! #[derive(Serialize, Deserialize)]
//! enum Message {
//!     Increment,
//!     Decrement,
//!     Reset,
//! }
//!
//! impl App for Hello {
//!     // Use the above Message as message
//!     type Message = Message;
//!     // Use the above Model as model
//!     type Model = Model;
//!     type ViewModel = String;
//!
//!     fn update(&self, message: Message, model: &mut Model) -> Vec<Command<Msg>> {
//!         match message {
//!             Message::Increment => model.count += 1,
//!             Message::Decrement => model.count -= 1,
//!             Message::Reset => model.count = 0,
//!         };
//!         vec![Command::Render]
//!     }
//!
//!     fn view(&self, model: &Model) -> ViewModel {
//!         format!("Count is: {}", model.count)
//!     }
//! }
//! ```
//!
//! ## Integrating with a Shell
//!
//! To use the application in a user interface shell, you need to expose the core interface for FFI
//!
//! ```rust
//! src/lib.rs
//!
//! use lazy_static::lazy_static;
//! use wasm_bindgen::prelude::wasm_bindgen;
//! use crux::Core;
//!
//! uniffi_macros::include_scaffolding!("hello");
//!
//! lazy_static! {
//!     static ref CORE: Core<Hello> = Core::new();
//! }
//!
//! #[wasm_bindgen]
//! pub fn message(data: &[u8]) -> Vec<u8> {
//!     CORE.message(data)
//! }
//!
//! #[wasm_bindgen]
//! pub fn response(data: &[u8]) -> Vec<u8> {
//!     CORE.response(data)
//! }
//!
//! #[wasm_bindgen]
//! pub fn view() -> Vec<u8> {
//!     CORE.view()
//! }
//! ```
//!
//! You will also need an `hello.udl` file describing the interface:
//!
//! ```
//! namespace hello {
//!   sequence<u8> message([ByRef] sequence<u8> msg);
//!   sequence<u8> response([ByRef] sequence<u8> res);
//!   sequence<u8> view();
//! };
//! ```
//!
//! Finally, you will need to set up the type generation for the `Model`, `Message` and `ViewModel` types.
//! See [typegen] for details.
//!

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
