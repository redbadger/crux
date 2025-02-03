use std::{future::Future, marker::PhantomData};

use crux_core::{command::RequestBuilder, Command, Request};

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
    pub fn get() -> RequestBuilder<Effect, Event, impl Future<Output = PlatformResponse>> {
        Command::request_from_shell(PlatformRequest)
    }
}
