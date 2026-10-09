#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unsafe_derive_deserialize)]

extern crate alloc;

pub mod app;
pub mod sse;

pub use app::*;
pub use crux_core::Core;
pub use crux_http as http;
