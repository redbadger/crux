//! A demo capability to get a name of the current platform

#[cfg(feature = "native_bridge")]
uniffi::setup_scaffolding!();

pub mod command;
pub mod protocol;

use std::marker::PhantomData;

use crux_core::{Command, Request, command::RequestBuilder};

pub use protocol::*;

pub struct Platform<Effect, Event> {
    effect: PhantomData<Effect>,
    event: PhantomData<Event>,
}

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
