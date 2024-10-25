use crux_core::capability::{CapabilityContext, Operation};
use crux_core::Command;
use futures::StreamExt;
use serde::{Deserialize, Serialize};

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
pub struct PubSub {
    context: CapabilityContext<PubSubOperation>,
}

impl PubSub {
    pub fn new(context: CapabilityContext<PubSubOperation>) -> Self {
        Self { context }
    }

    pub fn subscribe<F, Ev>(&self, make_event: F) -> Command<Ev>
    where
        F: FnOnce(Vec<u8>) -> Ev + Clone + Send + 'static,
    {
        let context = self.context.clone();
        let stream = context.stream_from_shell(PubSubOperation::Subscribe);
        Command::stream(stream.map(move |message| {
            let make_event = make_event.clone();
            Command::event(make_event(message.0))
        }))
    }

    pub fn publish<Ev>(&self, data: Vec<u8>) -> Command<Ev> {
        let ctx = self.context.clone();
        Command::empty_effect(async move { ctx.notify_shell(PubSubOperation::Publish(data)).await })
    }
}
