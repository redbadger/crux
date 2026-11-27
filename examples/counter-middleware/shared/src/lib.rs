#![allow(clippy::unsafe_derive_deserialize)]
// `boltffi::scaffolding!()` emits an empty enum.
#![allow(clippy::empty_enums)]
pub mod app;
mod capabilities;
#[cfg(feature = "ffi")]
mod ffi;
#[cfg(all(feature = "ffi", not(target_family = "wasm")))]
mod middleware;

#[cfg(feature = "ffi")]
boltffi::scaffolding!();

pub use crux_http as http;

pub use app::*;
pub use capabilities::{RandomNumber, RandomNumberRequest, sse};
pub use crux_core::Core;

#[cfg(feature = "ffi")]
pub use ffi::CoreFfi;
