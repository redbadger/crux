#![allow(clippy::unsafe_derive_deserialize)]
// `boltffi::scaffolding!()` emits an empty enum.
#![allow(clippy::empty_enums)]

mod app;
pub mod ffi;

boltffi::scaffolding!();

pub use app::*;
pub use crux_core::Core;
