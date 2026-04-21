//! Leptos components for the Weather shell.
//!
//! Every component reads its own slice of the Crux `ViewModel` and sends
//! user intent back through the shared `DispatchContext`. The contract with
//! the core is: read state through Leptos signals, write intent through a
//! callback. Signals are not used for events — see `lib.rs` for the rationale.

pub mod common;
pub mod favorites;
pub mod home;
pub mod onboard;

use leptos::callback::UnsyncCallback;
use leptos::prelude::expect_context;

use shared::Event;

// ANCHOR: dispatch
/// A callback that sends events to the Crux core.
///
/// `UnsyncCallback` (rather than `Callback`) because `Rc<shared::Core<Weather>>`
/// is `!Send` — WASM is single-threaded, so we never cross a thread boundary
/// and don't need `Arc` / `Send` / `Sync`.
pub type SendEvent = UnsyncCallback<Event>;

/// Context wrapper for the global dispatcher.
///
/// Provided once in `App` via `provide_context` and read anywhere in the tree
/// via [`use_dispatch`]. Avoids threading a `WriteSignal<Event>` through every
/// component's prop list.
#[derive(Clone)]
pub struct DispatchContext(pub SendEvent);

/// Pull the dispatcher from component context.
#[must_use]
pub fn use_dispatch() -> SendEvent {
    expect_context::<DispatchContext>().0
}
// ANCHOR_END: dispatch
