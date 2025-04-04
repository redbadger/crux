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
//! native language (e.g. Swift or Kotlin). It is also possible to use Crux from Rust shells.
//! The Core architecture is based on [Elm architecture](https://guide.elm-lang.org/architecture/).
//!
//! Quick glossary of terms to help you follow the example:
//!
//! * Core - the shared core written in Rust
//!
//! * Shell - the native side of the app on each platform handling UI and executing side effects
//!
//! * App - the main module of the core containing the application logic, especially model changes
//!   and side-effects triggered by events. An App can delegate to child apps, mapping Events and Effects.
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
//!   when an effect is completed. For example, a HTTP client is a capability.
//!
//! * Command - A description of a side-effect to be executed by the shell. Commands can be combined
//!   (synchronously with combinators, or asynchronously with Rust async) to run
//!   sequentially or concurrently, or any combination thereof.
//!
//! Below is a minimal example of a Crux-based application Core:
//!
//! ```rust
//!// src/app.rs
//!use crux_core::{render::{self, Render}, App, macros::Effect, Command};
//!use serde::{Deserialize, Serialize};
//!
//!// Model describing the application state
//!#[derive(Default)]
//!struct Model {
//!    count: isize,
//!}
//!
//!// Event describing the actions that can be taken
//!#[derive(Serialize, Deserialize)]
//!pub enum Event {
//!    Increment,
//!    Decrement,
//!    Reset,
//!}
//!
//!// Capabilities listing the side effects the Core
//!// will use to request side effects from the Shell
//!#[cfg_attr(feature = "typegen", derive(crux_core::macros::Export))]
//!#[derive(Effect)]
//!pub struct Capabilities {
//!    pub render: Render<Event>,
//!}
//!
//!#[derive(Default)]
//!struct Hello;
//!
//!impl App for Hello {
//!    // Use the above Event
//!    type Event = Event;
//!    // Use the above Model
//!    type Model = Model;
//!    type ViewModel = String;
//!    // Use the above Capabilities
//!    type Capabilities = Capabilities;
//!    // Use the above generated Effect
//!    type Effect = Effect;
//!
//!    fn update(&self, event: Event, model: &mut Model, caps: &Capabilities) -> Command<Effect, Event> {
//!        match event {
//!            Event::Increment => model.count += 1,
//!            Event::Decrement => model.count -= 1,
//!            Event::Reset => model.count = 0,
//!        };
//!
//!        // Request a UI update
//!        render::render()
//!    }
//!
//!    fn view(&self, model: &Model) -> Self::ViewModel {
//!        format!("Count is: {}", model.count)
//!    }
//!}
//! ```
//!
//! ## Integrating with a Shell
//!
//! To use the application in a user interface shell, you need to expose the core interface for FFI.
//! This "plumbing" will likely be simplified with macros in the future versions of Crux.
//!
//! ```rust,ignore
//! // src/lib.rs
//! pub mod app;
//!
//! use lazy_static::lazy_static;
//! use wasm_bindgen::prelude::wasm_bindgen;
//!
//! pub use crux_core::bridge::{Bridge, Request};
//! pub use crux_core::Core;
//! pub use crux_http as http;
//!
//! pub use app::*;
//!
//! uniffi_macros::include_scaffolding!("hello");
//!
//! lazy_static! {
//!     static ref CORE: Bridge<Effect, App> = Bridge::new(Core::new::<Capabilities>());
//! }
//!
//! #[wasm_bindgen]
//! pub fn process_event(data: &[u8]) -> Vec<u8> {
//!     CORE.process_event(data)
//! }
//!
//! #[wasm_bindgen]
//! pub fn handle_response(id: u32, data: &[u8]) -> Vec<u8> {
//!     CORE.handle_response(id, data)
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
//! See [typegen](https://docs.rs/crux_core/latest/crux_core/typegen/index.html) for details.
//!

pub mod bridge;
pub mod capability;
pub mod command;
pub mod testing;
#[cfg(feature = "typegen")]
pub mod typegen;

mod capabilities;
mod core;

use serde::Serialize;

pub use self::{
    capabilities::*,
    capability::{Capability, WithContext},
    command::Command,
    core::{Core, Effect, Request, ResolveError},
};
pub use crux_macros as macros;

/// Implement [`App`] on your type to make it into a Crux app. Use your type implementing [`App`]
/// as the type argument to [`Core`] or [`Bridge`](bridge::Bridge).
pub trait App: Default {
    /// Event, typically an `enum`, defines the actions that can be taken to update the application state.
    type Event: Unpin + Send + 'static;
    /// Model, typically a `struct` defines the internal state of the application
    type Model: Default;
    /// ViewModel, typically a `struct` describes the user interface that should be
    /// displayed to the user
    type ViewModel: Serialize;
    /// Capabilities, usually a `struct`, lists the capabilities used by this application.
    ///
    /// Typically, Capabilities should contain at least an instance of the built-in [`Render`](crate::render::Render) capability.
    ///
    /// Note: this `Capabilities` associated type will be deprecated soon as part of the completion of
    /// the migration to the new [`Command`](command) API.
    type Capabilities;
    /// Effect, the enum carrying effect requests created by capabilities.
    /// Normally this type is derived from `Capabilities` using the `crux_macros::Effect` derive macro
    type Effect: Effect + Unpin;

    /// Update method defines the transition from one `model` state to another in response to an `event`.
    ///
    /// `update` may mutate the `model` and returns a [`Command`] describing
    /// the managed side-effects to perform as a result of the `event`. Commands can be constructed by capabilities
    /// and combined to run sequentially or concurrently. If migrating from previous version of crux, you
    /// can return `Command::done()` for compatibility.
    ///
    /// For backwards compatibility, `update` may also use the capabilities provided by the `caps` argument
    /// to instruct the shell to perform side-effects. The side-effects will run concurrently (capability
    /// calls behave the same as go routines in Go or Promises in JavaScript) with each other and any
    /// effects captured by the returned `Command`. Capability calls don't return anything, but may
    /// take a `callback` event which should be dispatched when the effect completes.
    ///
    /// Typically, `update` should call at least [`Render::render`](crate::render::Render::render).
    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        caps: &Self::Capabilities,
    ) -> Command<Self::Effect, Self::Event>;

    /// View method is used by the Shell to request the current state of the user interface
    fn view(&self, model: &Self::Model) -> Self::ViewModel;
}
