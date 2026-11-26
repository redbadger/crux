#![allow(clippy::unsafe_derive_deserialize)]
// `boltffi::scaffolding!()` emits an empty enum.
#![allow(clippy::empty_enums)]
pub mod app;
pub mod capabilities;
mod ffi;

boltffi::scaffolding!();

pub use app::*;
pub use crux_core::Core;
pub use crux_kv as kv;

pub use ffi::CoreFfi;
