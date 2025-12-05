pub mod app;
#[cfg(any(feature = "wasm_bindgen", feature = "uniffi"))]
mod ffi;

pub use app::*;
pub use crux_core::Core;
pub use crux_http as http;
pub use crux_kv as key_value;
pub use crux_platform as platform;
pub use crux_time as time;

#[cfg(any(feature = "wasm_bindgen", feature = "uniffi"))]
pub use ffi::CoreFFI;

#[cfg(feature = "uniffi")]
const _: () = assert!(
    uniffi::check_compatible_version("0.30.0"),
    "please use uniffi v0.30.0"
);
#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();
