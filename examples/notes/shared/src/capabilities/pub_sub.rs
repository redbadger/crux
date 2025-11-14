use std::{future::Future, marker::PhantomData};

use futures::Stream;
use serde::{Deserialize, Serialize};

use crux_core::{
    capability::Operation,
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

pub struct PubSub<Effect, Event> {
    effect: PhantomData<Effect>,
    event: PhantomData<Event>,
}

impl<Effect, Event> PubSub<Effect, Event>
where
    Event: Send + 'static,
{
    #[must_use]
    pub fn subscribe() -> StreamBuilder<Effect, Event, impl Stream<Item = Vec<u8>>>
    where
        Effect: From<Request<PubSubOperation>> + Send + 'static,
    {
        Command::stream_from_shell(PubSubOperation::Subscribe).map(|Message(data)| data)
    }

    #[must_use]
    pub fn publish(data: Vec<u8>) -> NotificationBuilder<Effect, Event, impl Future<Output = ()>>
    where
        Effect: From<Request<PubSubOperation>> + Send + 'static,
    {
        Command::notify_shell(PubSubOperation::Publish(data))
    }
}
