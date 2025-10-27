#![deny(clippy::pedantic)]
//! A demo capability to get a name of the current platform

pub mod command;

use std::marker::PhantomData;

use crux_core::{Command, Request, capability::Operation, command::RequestBuilder};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformRequest;

// TODO revisit this
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformResponse(pub String);

impl Operation for PlatformRequest {
    type Output = PlatformResponse;
}

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
