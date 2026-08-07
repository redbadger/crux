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
//! * Command - A description of a side-effect or a sequence of side-effects to be executed by the shell.
//!   Commands can be combined (synchronously with combinators, or asynchronously with Rust async) to run
//!   sequentially or concurrently, or any combination thereof.
//!
//! * Capability - A user-friendly API used to create Commands for a specific effect type (e.g. HTTP)
//!
//!
//! Below is a minimal example of a Crux-based application Core:
//!
//! ```rust
//!// src/app.rs
//!use crux_core::{render::{self, RenderOperation}, App, macros::effect, Command};
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
//!// Effects the Core will request from the Shell
//!#[effect(typegen)]
//!pub enum Effect {
//!    Render(RenderOperation),
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
//!    // Use the above generated Effect
//!    type Effect = Effect;
//!
//!    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
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
//! To use the application from a shell, wrap the [`Core`] in a [`Bridge`](crate::bridge::Bridge),
//! which presents the same interface in serialized form, so that events, effect requests and the
//! view model can cross the FFI boundary as bytes.
//!
//! ```rust
//! # use crux_core::{render::{self, RenderOperation}, App, macros::effect, Command};
//! # use serde::{Deserialize, Serialize};
//! # #[derive(Default)]
//! # struct Model {
//! #     count: isize,
//! # }
//! # #[derive(Serialize, Deserialize)]
//! # pub enum Event {
//! #     Increment,
//! # }
//! # #[effect(typegen)]
//! # pub enum Effect {
//! #     Render(RenderOperation),
//! # }
//! # #[derive(Default)]
//! # struct Hello;
//! # impl App for Hello {
//! #     type Event = Event;
//! #     type Model = Model;
//! #     type ViewModel = String;
//! #     type Effect = Effect;
//! #     fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
//! #         match event {
//! #             Event::Increment => model.count += 1,
//! #         };
//! #         render::render()
//! #     }
//! #     fn view(&self, model: &Model) -> Self::ViewModel {
//! #         format!("Count is: {}", model.count)
//! #     }
//! # }
//! // src/ffi.rs
//! use crux_core::{
//!     Core,
//!     bridge::{Bridge, EffectId},
//! };
//!
//! pub struct CoreFfi {
//!     core: Bridge<Hello>,
//! }
//!
//! impl CoreFfi {
//!     pub fn new() -> Self {
//!         Self {
//!             core: Bridge::new(Core::new()),
//!         }
//!     }
//!
//!     /// Send an event to the app, returning the serialized effect requests it caused.
//!     pub fn update(&self, event: &[u8]) -> Vec<u8> {
//!         let mut requests = vec![];
//!         self.core
//!             .update(event, &mut requests)
//!             .expect("event should deserialize");
//!
//!         requests
//!     }
//!
//!     /// Resolve an effect request with the shell's output, returning any follow-up requests.
//!     pub fn resolve(&self, id: u32, output: &[u8]) -> Vec<u8> {
//!         let mut requests = vec![];
//!         self.core
//!             .resolve(EffectId(id), output, &mut requests)
//!             .expect("output should deserialize");
//!
//!         requests
//!     }
//!
//!     /// Get the current view model, serialized.
//!     pub fn view(&self) -> Vec<u8> {
//!         let mut view = vec![];
//!         self.core.view(&mut view).expect("view model should serialize");
//!
//!         view
//!     }
//! }
//! ```
//!
//! The three methods above are the entire interface the shell sees. In a real app you would
//! handle the errors rather than panicking on them.
//!
//! The bindings which let Swift, Kotlin, TypeScript or C# call those methods are generated by
//! [BoltFFI](https://www.boltffi.dev/). Annotate the `impl` block with `#[boltffi::export]`,
//! describe your targets in a `boltffi.toml`, and run `boltffi pack apple` (or `android`, `wasm`)
//! to build the library and generate the foreign code that calls it:
//!
//! ```rust,ignore
//! #[boltffi::export]
//! impl CoreFfi {
//!     // ...as above
//! }
//! ```
//!
//! ## Type generation
//!
//! The shell also needs its own definitions of the types crossing the boundary — `Event`,
//! `ViewModel` and the effect payloads. These are generated separately from the FFI bindings,
//! by deriving [`Facet`](https://docs.rs/facet) on those types and running a `codegen` binary
//! against them, behind the `facet_typegen` feature. See
//! [`type_generation::facet`](https://docs.rs/crux_core/latest/crux_core/type_generation/facet/index.html)
//! for details.
//!
//! The [`counter` example](https://github.com/redbadger/crux/tree/master/examples/counter) shows
//! all of this end to end, with shells written in Swift, Kotlin, TypeScript, C# and Rust.
//!

pub mod bridge;
pub mod capability;
pub mod command;
pub mod effects;
pub mod middleware;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
#[cfg(any(feature = "typegen", feature = "facet_typegen"))]
pub mod type_generation;

#[doc(hidden)]
#[macro_export]
#[cfg(any(test, feature = "testing"))]
macro_rules! __crux_core_testing_items {
    ($($tokens:tt)*) => {
        $($tokens)*
    };
}

#[doc(hidden)]
#[macro_export]
#[cfg(not(any(test, feature = "testing")))]
macro_rules! __crux_core_testing_items {
    ($($tokens:tt)*) => {};
}

mod capabilities;
mod core;

pub use capabilities::*;
pub use command::Command;
pub use core::{Core, Effect, EffectFFI, Request, RequestHandle, Resolvable, ResolveError};
#[cfg(feature = "uniffi_compat_bindgen")]
#[deprecated(
    since = "0.19.0",
    note = "UniFFI bindgen support is deprecated; use BoltFFI package/generate commands instead"
)]
pub mod bindgen;
#[cfg(feature = "default")]
pub use crux_macros as macros;
#[cfg(feature = "typegen")]
pub use type_generation::serde as typegen;

/// Implement [`App`] on your type to make it into a Crux app. Use your type implementing [`App`]
/// as the type argument to [`Core`] or [`Bridge`](crate::bridge::Bridge).
pub trait App {
    /// `Event`, typically an `enum`, defines the actions that can be taken to update the application state.
    type Event: Unpin + Send + 'static;
    /// `Model`, typically a `struct` defines the internal state of the application
    type Model;
    /// `ViewModel`, typically a `struct` describes the user interface that should be
    /// displayed to the user
    type ViewModel;
    /// `Effect`, the enum carrying the effect requests the app can make of the shell.
    /// Normally this type is written with the `crux_macros::effect` attribute macro,
    /// which implements the necessary traits for you.
    type Effect: Effect + Unpin;

    /// Update method defines the transition from one `model` state to another in response to an `event`.
    ///
    /// `update` may mutate the `model` and returns a [`Command`] describing
    /// the managed side-effects to perform as a result of the `event`. Commands are constructed by
    /// capabilities, and combined to run sequentially or concurrently. If an event requires no
    /// side-effects, return [`Command::done`].
    ///
    /// Typically, `update` should call at least [`render`](crate::render::render).
    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
    ) -> Command<Self::Effect, Self::Event>;

    /// View method is used by the Shell to request the current state of the user interface
    fn view(&self, model: &Self::Model) -> Self::ViewModel;
}
