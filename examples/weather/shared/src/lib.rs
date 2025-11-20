#![allow(clippy::missing_panics_doc)]

mod app;
mod config;
mod favorites;
#[cfg(any(feature = "wasm_bindgen", feature = "uniffi"))]
mod ffi;
mod location;
mod navigation;
mod weather;

pub use app::App;

#[cfg(any(feature = "wasm_bindgen", feature = "uniffi"))]
pub use ffi::CoreFFI;

#[cfg(feature = "uniffi")]
const _: () = assert!(
    uniffi::check_compatible_version("0.29.4"),
    "please use uniffi v0.29.4"
);
#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();
