use std::future::Future;

use futures::Stream;
use serde::{Deserialize, Serialize};

use crux_core::{
    capability::{CapabilityContext, Operation},
    command::{NotificationBuilder, StreamBuilder},
    Command, Request,
};

// TODO add topics

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum PubSubOperation {
    Publish(Vec<u8>),
    Subscribe,
}

#[derive(Deserialize)]
pub struct Message(pub Vec<u8>);

impl Operation for PubSubOperation {
    type Output = Message;
}

#[derive(crux_core::macros::Capability)]
pub struct PubSub<Event> {
    context: CapabilityContext<PubSubOperation, Event>,
}

impl<Event> PubSub<Event>
where
    Event: Send + 'static,
{
    pub fn new(context: CapabilityContext<PubSubOperation, Event>) -> Self {
        Self { context }
    }

    pub fn subscribe<Effect>() -> StreamBuilder<Effect, Event, impl Stream<Item = Vec<u8>>>
    where
        Effect: From<Request<PubSubOperation>> + Send + 'static,
    {
        Command::stream_from_shell(PubSubOperation::Subscribe).map(|Message(data)| data)
    }

    pub fn publish<Effect>(
        data: Vec<u8>,
    ) -> NotificationBuilder<Effect, Event, impl Future<Output = ()>>
    where
        Effect: From<Request<PubSubOperation>> + Send + 'static,
    {
        Command::notify_shell(PubSubOperation::Publish(data))
    }
}
