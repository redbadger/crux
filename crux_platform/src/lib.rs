#![allow(clippy::unsafe_derive_deserialize)]
//! # Deprecated
//!
//! This crate is deprecated and will no longer be maintained.
//!
//! The `Platform` capability was originally provided as a convenience for querying the current
//! platform from the shell. Create a custom capability in your own project instead —
//! see the [README](https://crates.io/crates/crux_platform) for a drop-in migration snippet.

pub mod command;
pub mod protocol;

use std::marker::PhantomData;

use crux_core::{Command, Request, command::RequestBuilder};

pub use protocol::*;

#[deprecated(
    since = "0.10.0",
    note = "The `crux_platform` crate is deprecated. Copy the types into your own project instead. See the README for migration guidance: https://crates.io/crates/crux_platform"
)]
pub struct Platform<Effect, Event> {
    effect: PhantomData<Effect>,
    event: PhantomData<Event>,
}

#[allow(deprecated)]
impl<Effect, Event> Platform<Effect, Event>
where
    Effect: From<Request<PlatformRequest>> + Send + 'static,
    Event: Send + 'static,
{
    /// Get the platform of the shell
    #[must_use]
    pub fn get() -> RequestBuilder<Effect, Event, impl Future<Output = PlatformResponse>> {
        Command::request_from_shell(PlatformRequest)
    }
}
