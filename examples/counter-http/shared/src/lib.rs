#![allow(clippy::unsafe_derive_deserialize)]
// `boltffi::scaffolding!()` emits an empty enum.
#![allow(clippy::empty_enums)]
pub mod app;
mod ffi;
pub mod sse;

boltffi::scaffolding!();

pub use app::*;
pub use crux_core::Core;
pub use crux_http as http;

pub use ffi::CoreFfi;
