use std::{future::Future, marker::PhantomData};

use crux_core::{Command, Request, command::RequestBuilder};

use crate::{PlatformRequest, PlatformResponse};

pub struct Platform<Effect, Event> {
    effect: PhantomData<Effect>,
    event: PhantomData<Event>,
}

impl<Effect, Event> Platform<Effect, Event>
where
    Effect: From<Request<PlatformRequest>> + Send + 'static,
    Event: Send + 'static,
{
    #[must_use]
    pub fn get() -> RequestBuilder<Effect, Event, impl Future<Output = PlatformResponse>> {
        Command::request_from_shell(PlatformRequest)
    }
}
