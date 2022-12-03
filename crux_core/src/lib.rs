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
//! The Core architecture is based on [Elm architecture](https://guide.elm-lang.org/architecture/).
//!
//! Below is a minimal example of a Crux based application Core:
//!
//! ```rust,ignore
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
//! ```rust,ignore
//! // src/lib.rs
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
//! ```ignore
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

pub mod capability;
pub mod command;
mod continuations;
pub mod render;
#[cfg(feature = "typegen")]
pub mod typegen;

// TODO: should this be public?  Not sure...
pub mod channels;

pub use capability::{Capabilities, Capability, CapabilityFactory};
use command::Callback;
pub use command::Command;
use continuations::ContinuationStore;
use serde::{Deserialize, Serialize};
use std::{
    marker::PhantomData,
    sync::{Mutex, RwLock},
};

/// Implement [App] on your type to make it into a Crux app. Use your type implementing [App]
/// as the type argument to [Core].
pub trait App: Default {
    /// Model, typically a `struct` defines the internal state of the application
    type Model: Default;
    /// Message, typically an `enum`, defines the actions that can be taken to update the application state.
    type Event: 'static;
    /// ViewModel, typically a `struct` describes the user interface that should be
    /// displayed to the user
    type ViewModel: Serialize;

    type Capabilities;

    /// Update method defines the transition from one `model` state to another in response to a `msg`.
    ///
    /// Update function can return a list of [`Command`]s, instructing the shell to perform side-effects.
    /// Typically, the function should return at least [`Command::render`] to update the user interface.
    fn update(&self, msg: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities);

    /// View method is used by the Shell to request the current state of the user interface
    fn view(&self, model: &Self::Model) -> Self::ViewModel;
}
/// The Crux core. Create an instance of this type with your app as the type parameter
pub struct Core<Ef, A>
where
    A: App,
{
    model: RwLock<A::Model>,
    continuations: ContinuationStore<A::Event>,
    capabilities: A::Capabilities,
    command_receiver: crate::channels::Receiver<Command<Ef, A::Event>>,
    app: A,
    _marker: PhantomData<Ef>,
}

impl<Ef, A> Core<Ef, A>
where
    Ef: Serialize + Send + 'static,
    A: App,
{
    /// Create an instance of the Crux core to start a Crux application, e.g.
    ///
    /// ```rust,ignore
    /// lazy_static! {
    ///     static ref CORE: Core<Hello> = Core::new();
    /// }
    /// ```
    ///
    /// The core interface passes across messages serialized as bytes. These can be
    /// deserialized using the types generated using the [typegen] module.
    pub fn new<CapFactory>() -> Self
    where
        CapFactory: CapabilityFactory<A, Ef>,
    {
        let (sender, receiver) = crate::channels::channel();
        Self {
            model: Default::default(),
            continuations: Default::default(),
            app: Default::default(),
            capabilities: CapFactory::build(sender),
            command_receiver: receiver,
            _marker: PhantomData,
        }
    }

    /// Receive a message from the shell.
    ///
    /// The `msg` is serialized and will be deserialized by the core.
    pub fn message<'de>(&self, msg: &'de [u8]) -> Vec<u8>
    where
        <A as App>::Event: Deserialize<'de>,
    {
        let msg: <A as App>::Event = bcs::from_bytes(msg).expect("Message deserialization failed.");

        let mut model = self.model.write().expect("Model RwLock was poisoned.");

        self.app.update(msg, &mut model, &self.capabilities);

        let requests = self
            .command_receiver
            .drain()
            .map(|c| self.continuations.pause(c))
            .collect::<Vec<_>>();

        bcs::to_bytes(&requests).expect("Request serialization failed.")
    }

    /// Receive a response to a capability request from the shell.
    ///
    /// The `res` is serialized and will be deserialized by the core. The `uuid`  field of
    /// the deserialized [`Response`] MUST match the `uuid` of the [`Request`] which
    /// triggered it, else the core will panic.
    pub fn response<'de>(&self, uuid: &[u8], body: &'de [u8]) -> Vec<u8>
    where
        <A as App>::Event: Deserialize<'de>,
    {
        let msg = self.continuations.resume(uuid, body.to_owned()); // FIXME is this to_owned the right fix?

        let mut model = self.model.write().expect("Model RwLock was poisoned.");

        self.app.update(msg, &mut model, &self.capabilities);

        let requests = self
            .command_receiver
            .drain()
            .map(|c| self.continuations.pause(c))
            .collect::<Vec<_>>();

        bcs::to_bytes(&requests).expect("Request serialization failed.")
    }

    /// Get the current state of the app's view model (serialized).
    pub fn view(&self) -> Vec<u8> {
        let model = self.model.read().expect("Model RwLock was poisoned.");

        let value = self.app.view(&model);
        bcs::to_bytes(&value).expect("View model serialization failed.")
    }
}

/// Request for a side-effect passed from the Core to the Shell. The `uuid` links
/// the `Request` with the corresponding [`Response`] to pass the data back
/// to the [`App::update`] function wrapped in the correct `Message`.
#[derive(Serialize, Deserialize)]
pub struct Request<Effect> {
    pub uuid: Vec<u8>,
    pub effect: Effect,
}
