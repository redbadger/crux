#![allow(clippy::unsafe_derive_deserialize)]
pub mod app;
mod capabilities;
#[cfg(feature = "ffi")]
mod ffi;
#[cfg(all(feature = "ffi", not(target_family = "wasm")))]
mod rng_handler;

pub use crux_http as http;

pub use app::*;
pub use capabilities::{RandomNumber, RandomNumberRequest, sse};
pub use crux_core::Core;

#[cfg(feature = "ffi")]
pub use ffi::CoreFFI;
