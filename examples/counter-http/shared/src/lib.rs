pub mod app;
#[cfg(any(feature = "wasm_bindgen", feature = "uniffi"))]
mod ffi;
pub mod sse;

pub use app::*;
pub use crux_core::Core;
pub use crux_http as http;

#[cfg(any(feature = "wasm_bindgen", feature = "uniffi"))]
pub use ffi::CoreFFI;

#[cfg(feature = "uniffi")]
const _: () = assert!(
    uniffi::check_compatible_version("0.29.4"),
    "please use uniffi v0.29.4"
);
#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();
