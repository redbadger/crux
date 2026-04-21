//! Shared business logic for the Crux weather example.
//!
//! This crate is the portable core of the app: a pure `update`/`view` pipeline
//! built on [Crux](https://crux-rs.github.io/). The native and web shells
//! (`apple/`, `android/`, `web-leptos/`, `web-nextjs/`) drive it through
//! `CoreFFI` (behind `uniffi` or `wasm_bindgen` features) and render the
//! resulting [`ViewModel`].
//!
//! # Where to start
//!
//! - [`app::Weather`] — the `App` impl; the minimal surface that ties the
//!   core together.
//! - [`model`] — the state machine. The top-level [`Model`](model::Model)
//!   moves between `Initializing`, `Onboard`, `Active`, and `Failed`; each
//!   stage has a nested model and event type.
//! - [`view`] — `ViewModel` and its children. The shape the shell renders.
//! - [`effects`] — the [`Effect`] enum plus the custom capabilities
//!   ([`location`](effects::location), [`secret`](effects::secret)) and
//!   HTTP clients ([`http`](effects::http)) the model uses.
//!
//! Each nested model is a small state machine that returns an `Outcome`
//! from its `update` — either continuing with a new state or completing
//! with a transition the parent acts on.
//!
//! For a narrative walkthrough and diagrams, see the example's
//! [ARCHITECTURE.md](https://github.com/redbadger/crux/blob/master/examples/weather/ARCHITECTURE.md).

#![allow(clippy::unsafe_derive_deserialize)]
#![allow(clippy::missing_panics_doc)]
pub mod app;
pub mod effects;
#[cfg(any(feature = "wasm_bindgen", feature = "uniffi"))]
mod ffi;
pub mod model;
pub mod view;

pub use app::*;
pub use crux_core::Core;
pub use crux_http as http;
pub use crux_kv as kv;
pub use effects::Effect;
pub use model::Event;
pub use view::ViewModel;

#[cfg(any(feature = "wasm_bindgen", feature = "uniffi"))]
pub use ffi::CoreFFI;

#[cfg(feature = "uniffi")]
const _: () = assert!(
    uniffi::check_compatible_version("0.29.4"),
    "please use uniffi v0.29.4"
);
#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();
