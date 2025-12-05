pub mod app;
mod capabilities;
mod ffi;
#[cfg(feature = "uniffi")]
mod middleware;

pub use crux_http as http;

pub use app::*;
pub use capabilities::{RandomNumber, RandomNumberRequest, sse};
pub use crux_core::Core;

#[cfg(feature = "uniffi")]
const _: () = assert!(
    uniffi::check_compatible_version("0.30.0"),
    "please use uniffi v0.30.0"
);
#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

#[cfg(feature = "uniffi")]
pub use ffi::uniffi::CoreFFI;

#[cfg(feature = "wasm_bindgen")]
pub use ffi::wasm_bindgen::CoreFFI as WasmCoreFFI;
