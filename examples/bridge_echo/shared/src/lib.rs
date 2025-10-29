pub mod app;
mod ffi;

pub use crux_core::{bridge::Bridge, Core, Request};

pub use app::*;

#[cfg(not(target_family = "wasm"))]
const _: () = assert!(
    uniffi::check_compatible_version("0.29.4"),
    "please use uniffi v0.29.4"
);
#[cfg(not(target_family = "wasm"))]
uniffi::setup_scaffolding!();

#[cfg(not(target_family = "wasm"))]
pub use ffi::uniffi_ffi::CoreFFI;

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub use ffi::wasm_ffi::CoreFFI;
