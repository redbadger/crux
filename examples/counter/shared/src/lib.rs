#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unsafe_derive_deserialize)]

extern crate alloc;

// ANCHOR: lib
mod app;

pub use app::*;
pub use crux_core::Core;
// ANCHOR_END: lib
