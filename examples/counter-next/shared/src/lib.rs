pub mod app;
mod capabilities;
#[cfg(any(feature = "uniffi", feature = "wasm_bindgen", feature = "wit_bindgen"))]
mod ffi;
#[cfg(feature = "uniffi")]
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

#[cfg(feature = "uniffi")]
pub use ffi::uniffi::CoreFFI;

#[cfg(feature = "wasm_bindgen")]
pub use ffi::wasm_bindgen::CoreFFI as WasmCoreFFI;

#[cfg(feature = "wit_bindgen")]
pub use ffi::wit_bindgen::CoreFFI as WitCoreFFI;
