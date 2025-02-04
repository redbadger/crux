use std::future::Future;

use serde::{Deserialize, Serialize};

use crux_core::{
    capability::{CapabilityContext, Operation},
    command::RequestBuilder,
    Command, Request,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum DelayOperation {
    Start { millis: usize },
}

impl Operation for DelayOperation {
    type Output = ();
}

#[derive(crux_core::macros::Capability)]
pub struct Delay<Event> {
    context: CapabilityContext<DelayOperation, Event>,
}

impl<Event> Delay<Event>
where
    Event: Send + 'static,
{
    pub fn new(context: CapabilityContext<DelayOperation, Event>) -> Self {
        Self { context }
    }

    pub fn start<Effect>(millis: usize) -> RequestBuilder<Effect, Event, impl Future<Output = ()>>
    where
        Effect: From<Request<DelayOperation>> + Send + 'static,
    {
        Command::request_from_shell(DelayOperation::Start { millis })
    }
}
