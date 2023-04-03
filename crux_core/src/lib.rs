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
//! Crux applications are split into two parts: a Core written in Rust and a Shell written in the platform
//! native language (e.g. Swift or Kotlin).
//! The Core architecture is based on [Elm architecture](https://guide.elm-lang.org/architecture/).
//!
//! Quick glossary of terms to help you follow the example:
//!
//! * Core - the shared core written in Rust
//!
//! * Shell - the native side of the app on each platform handling UI and executing side effects
//!
//! * App - the main module of the core containing the application logic, especially model changes
//!   and side-effects triggered by events. App can be composed from modules, each resembling a smaller, simpler app.
//!
//! * Event - main input for the core, typically triggered by user interaction in the UI
//!
//! * Model - data structure (typically tree-like) holding the entire application state
//!
//! * View model - data structure describing the current state of the user interface
//!
//! * Effect - A side-effect the core can request from the shell. This is typically a form of I/O or similar
//!   interaction with the host platform. Updating the UI is considered an effect.
//!
//! * Capability - A user-friendly API used to request effects and provide events that should be dispatched
//!   when the effect is completed. For example, a HTTP client is a capability.
//!
//! Below is a minimal example of a Crux-based application Core:
//!
//! ```rust,ignore
//! // src/app.rs
//!
//! use serde::{Serialize, Deserialize};
//! use crux_core::{App, render::Render};
//!
//! // Model describing the application state
//! #[derive(Default)]
//! struct Model {
//!     count: isize,
//! }
//!
//! // Event describing the actions that can be taken
//! #[derive(Serialize, Deserialize)]
//! enum Event {
//!     Increment,
//!     Decrement,
//!     Reset,
//! }
//!
//! // Capabilities listing the side effects the Core
//! // will use to request side effects from the Shell
//! #[derive(Effect)]
//! pub struct Capabilities {
//!     pub render: Render<Event>
//! }
//!
//! impl App for Hello {
//!     // Use the above Event
//!     type Event = Event;
//!     // Use the above Model
//!     type Model = Model;
//!     type ViewModel = String;
//!     // Use the above Capabilities
//!     type Capabilities = Capabilities;
//!
//!     fn update(&self, event: Event, model: &mut Model, caps: &Capabilities) {
//!         match event {
//!             Event::Increment => model.count += 1,
//!             Event::Decrement => model.count -= 1,
//!             Event::Reset => model.count = 0,
//!         };
//!
//!         // Request a UI update
//!         caps.render.render()
//!     }
//!
//!     fn view(&self, model: &Model) -> self::ViewModel {
//!         format!("Count is: {}", model.count)
//!     }
//! }
//! ```
//!
//! ## Integrating with a Shell
//!
//! To use the application in a user interface shell, you need to expose the core interface for FFI.
//! This "plumbing" will likely be simplified with macros in the next version of Crux.
//!
//! ```rust,ignore
//! // src/lib.rs
//! pub mod app;
//!
//! use lazy_static::lazy_static;
//! use wasm_bindgen::prelude::wasm_bindgen;
//!
//! pub use crux_core::Request;
//! use crux_core::Core;
//!
//! pub use app::*;
//!
//! uniffi_macros::include_scaffolding!("hello");
//!
//! lazy_static! {
//!     static ref CORE: Core<Effect, App> = Core::new::<Capabilities>();
//! }
//!
//! #[wasm_bindgen]
//! pub fn process_event(data: &[u8]) -> Vec<u8> {
//!     CORE.process_event(data)
//! }
//!
//! #[wasm_bindgen]
//! pub fn handle_response(uuid: &[u8], data: &[u8]) -> Vec<u8> {
//!     CORE.handle_response(uuid, data)
//! }
//!
//! #[wasm_bindgen]
//! pub fn view() -> Vec<u8> {
//!     CORE.view()
//! }
//! ```
//!
//! You will also need a `hello.udl` file describing the foreign function interface:
//!
//! ```ignore
//! // src/hello.udl
//! namespace hello {
//!   sequence<u8> process_event([ByRef] sequence<u8> msg);
//!   sequence<u8> handle_response([ByRef] sequence<u8> res);
//!   sequence<u8> view();
//! };
//! ```
//!
//! Finally, you will need to set up the type generation for the `Model`, `Message` and `ViewModel` types.
//! See [typegen] for details.
//!

pub mod bridge;
pub mod capability;
mod channels;
mod effect;
mod executor;
mod future;
pub mod render;
pub mod steps;
mod stream;
pub mod testing;
#[cfg(feature = "typegen")]
pub mod typegen;

use std::sync::RwLock;

use serde::Serialize;

pub use bridge::Bridge;
use capability::{Operation, ProtoContext};
use channels::Receiver;
pub use effect::Effect;
use executor::QueuingExecutor;
pub use steps::Step;

pub use self::{
    capability::{Capability, WithContext},
    future::ShellRequest,
    stream::ShellStream,
};

/// Implement [App] on your type to make it into a Crux app. Use your type implementing [App]
/// as the type argument to [Core].
pub trait App: Default {
    /// Event, typically an `enum`, defines the actions that can be taken to update the application state.
    type Event: Send + 'static;
    /// Model, typically a `struct` defines the internal state of the application
    type Model: Default;
    /// ViewModel, typically a `struct` describes the user interface that should be
    /// displayed to the user
    type ViewModel: Serialize;
    /// Capabilities, typically a `struct`, lists the capabilities used by this application
    /// Typically, Capabilities should contain at least an instance of the built-in [`Render`](crate::render::Render) capability.
    type Capabilities;

    /// Update method defines the transition from one `model` state to another in response to an `event`.
    ///
    /// Update function can mutate the `model` and use the capabilities provided by the `caps` argument
    /// to instruct the shell to perform side-effects. The side-effects will run concurrently (capability
    /// calls behave the same as go routines in Go or Promises in JavaScript). Capability calls
    /// don't return anything, but may take a `callback` event which should be dispatched when the
    /// effect completes.
    ///
    /// Typically, `update` should call at least [`Render::render`](crate::render::Render::render).
    fn update(&self, event: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities);

    /// View method is used by the Shell to request the current state of the user interface
    fn view(&self, model: &Self::Model) -> Self::ViewModel;
}

/// The Crux core. Create an instance of this type with your effect type, and your app type as type parameters
pub struct Core<Ef, A>
where
    A: App,
{
    model: RwLock<A::Model>,
    executor: QueuingExecutor,
    capabilities: A::Capabilities,
    steps: Receiver<Ef>,
    capability_events: Receiver<A::Event>,
    app: A,
}

impl<Ef, A> Core<Ef, A>
where
    Ef: Effect,
    A: App,
{
    /// Create an instance of the Crux core to start a Crux application, e.g.
    ///
    /// ```rust,ignore
    /// lazy_static! {
    ///     static ref CORE: Core<HelloEffect, Hello> = Core::new::<HelloCapabilities>();
    /// }
    /// ```
    ///
    /// The core interface passes across messages serialized as bytes. These can be
    /// deserialized in the Shell using the types generated using the [typegen] module.
    pub fn new<Capabilities>() -> Self
    where
        Capabilities: WithContext<A, Ef>,
    {
        let (step_sender, step_receiver) = crate::channels::channel();
        let (event_sender, event_receiver) = crate::channels::channel();
        let (executor, spawner) = executor::executor_and_spawner();
        let capability_context = ProtoContext::new(step_sender, event_sender, spawner);

        Self {
            model: Default::default(),
            executor,
            app: Default::default(),
            capabilities: Capabilities::new_with_context(capability_context),
            steps: step_receiver,
            capability_events: event_receiver,
        }
    }

    pub fn process_event(&self, event: <A as App>::Event) -> Vec<Ef> {
        let mut model = self.model.write().expect("Model RwLock was poisoned.");

        self.app.update(event, &mut model, &self.capabilities);

        self.process()
    }

    pub fn resolve_step<Op>(&self, step: &mut Step<Op>, result: Op::Output) -> Vec<Ef>
    where
        Op: Operation,
    {
        let _ = step.resolve(result);

        self.process()
    }

    pub(crate) fn process(&self) -> Vec<Ef> {
        self.executor.run_all();

        while let Some(capability_event) = self.capability_events.receive() {
            let mut model = self.model.write().expect("Model RwLock was poisoned.");
            self.app
                .update(capability_event, &mut model, &self.capabilities);
            drop(model);
            self.executor.run_all();
        }

        self.steps.drain().collect()
    }

    /// Get the current state of the app's view model (serialized).
    pub fn view(&self) -> <A as App>::ViewModel {
        let model = self.model.read().expect("Model RwLock was poisoned.");

        self.app.view(&model)
    }
}

impl<Ef, A> Default for Core<Ef, A>
where
    Ef: Effect,
    A: App,
    A::Capabilities: WithContext<A, Ef>,
{
    fn default() -> Self {
        Self::new::<A::Capabilities>()
    }
}
