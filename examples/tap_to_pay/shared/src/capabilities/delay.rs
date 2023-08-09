use crux_core::capability::{Capability, CapabilityContext, Operation};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum DelayOperation {
    Start { millis: usize },
}

impl Operation for DelayOperation {
    type Output = ();
}

pub struct Delay<Event> {
    context: CapabilityContext<DelayOperation, Event>,
}

impl<Ev> Delay<Ev>
where
    Ev: 'static + Send,
{
    pub fn new(context: CapabilityContext<DelayOperation, Ev>) -> Self {
        Self { context }
    }

    pub fn start(&self, millis: usize, event: Ev) {
        self.context.spawn({
            let context = self.context.clone();

            async move {
                context
                    .request_from_shell(DelayOperation::Start { millis })
                    .await;

                context.update_app(event);
            }
        })
    }
}

impl<Ev> Capability<Ev> for Delay<Ev> {
    type Operation = DelayOperation;
    type MappedSelf<MappedEv> = Delay<MappedEv>;

    fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
    where
        F: Fn(NewEvent) -> Ev + Send + Sync + Copy + 'static,
        Ev: 'static,
        NewEvent: 'static + Send,
    {
        Delay::new(self.context.map_event(f))
    }
}
