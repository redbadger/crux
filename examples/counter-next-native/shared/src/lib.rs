pub mod app;
mod capabilities;
mod ffi;
mod middleware;

pub use crux_http as http;

pub use app::*;
pub use capabilities::{RandomNumber, RandomNumberRequest, sse};
pub use crux_core::Core;

const _: () = assert!(
    uniffi::check_compatible_version("0.31.0"),
    "please use uniffi v0.31.0"
);
uniffi::setup_scaffolding!();

pub use ffi::native::CoreFFI;

// uniffi-bindgen-react-native generates TypeScript bindings for both
// React Native (iOS/Android via JSI) and Web (WASM) from the UniFFI annotations.
