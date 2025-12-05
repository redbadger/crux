mod app;
pub mod ffi;

pub use app::*;
pub use crux_core::Core;

#[cfg(feature = "uniffi")]
const _: () = assert!(
    uniffi::check_compatible_version("0.30.0"),
    "please use uniffi v0.30.0"
);
#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();
