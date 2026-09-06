//! A tiny publish/subscribe capability, one type per operation.
//!
//! Publishing is a notification — the shell broadcasts the bytes and there is
//! nothing to answer. Subscribing is a stream — the shell resolves the request
//! once per [`Message`] it receives from a peer, for as long as the
//! subscription lives.

use std::{future::Future, marker::PhantomData};

use crux_core::{
    Command, Request,
    command::{NotificationBuilder, StreamBuilder},
    macros::Operation,
};
use facet::Facet;
use futures::Stream;
use serde::{Deserialize, Serialize};

// TODO add topics

// ANCHOR: operations
/// Broadcast `bytes` to every peer. Nothing is expected in return.
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[operation(notify)]
pub struct Publish(pub Vec<u8>);

/// Receive every [`Message`] a peer publishes, until the subscription is
/// dropped.
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[operation(stream, output = Message)]
pub struct Subscribe;

/// One published payload, as it arrives from a peer.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Message(pub Vec<u8>);
// ANCHOR_END: operations

pub struct PubSub<Effect, Event> {
    effect: PhantomData<Effect>,
    event: PhantomData<Event>,
}

impl<Effect, Event> PubSub<Effect, Event>
where
    Event: Send + 'static,
{
    // ANCHOR: builders
    #[must_use]
    pub fn subscribe() -> StreamBuilder<Effect, Event, impl Stream<Item = Vec<u8>>>
    where
        Effect: From<Request<Subscribe>> + Send + 'static,
    {
        Command::stream_from_shell(Subscribe).map(|Message(data)| data)
    }

    #[must_use]
    pub fn publish(data: Vec<u8>) -> NotificationBuilder<Effect, Event, impl Future<Output = ()>>
    where
        Effect: From<Request<Publish>> + Send + 'static,
    {
        Command::notify_shell(Publish(data))
    }
    // ANCHOR_END: builders
}
