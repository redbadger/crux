#![allow(clippy::unsafe_derive_deserialize)]
pub mod app;
mod capabilities;
mod ffi;
#[cfg(not(target_family = "wasm"))]
mod middleware;

pub use crux_http as http;

pub use app::*;
pub use capabilities::{RandomNumber, RandomNumberRequest, sse};
pub use crux_core::Core;

#[cfg(feature = "uniffi")]
const _: () = assert!(
    uniffi::check_compatible_version("0.29.4"),
    "please use uniffi v0.29.4"
);
#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

pub use ffi::CoreFFI;

#[cfg(feature = "wasm_bindgen")]
pub use ffi::wasm_bindgen::CoreFFI as WasmCoreFFI;
