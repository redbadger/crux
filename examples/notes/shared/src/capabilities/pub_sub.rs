use crux_core::capability::{Capability, CapabilityContext, Operation};
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

pub struct PubSub<Event> {
    context: CapabilityContext<PubSubOperation, Event>,
}

impl<Ev> PubSub<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<PubSubOperation, Ev>) -> Self {
        Self { context }
    }

    pub fn subscribe<F>(&self, make_event: F)
    where
        F: Fn(Vec<u8>) -> Ev + Clone + Send + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();

            async move {
                let mut stream = context.stream_from_shell(PubSubOperation::Subscribe);

                while let Some(message) = stream.next().await {
                    let make_event = make_event.clone();

                    context.update_app(make_event(message.0));
                }
            }
        })
    }

    pub fn publish(&self, data: Vec<u8>) {
        self.context.spawn({
            let context = self.context.clone();

            async move {
                context.notify_shell(PubSubOperation::Publish(data)).await;
            }
        })
    }
}

impl<Ef> Capability<Ef> for PubSub<Ef> {
    type Operation = PubSubOperation;
    type MappedSelf<MappedEv> = PubSub<MappedEv>;

    fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
    where
        F: Fn(NewEvent) -> Ef + Send + Sync + Copy + 'static,
        Ef: 'static,
        NewEvent: 'static,
    {
        PubSub::new(self.context.map_event(f))
    }
}
