use crux_core::capability::{CapabilityContext, Operation};
use serde::{Deserialize, Serialize};

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

impl<Ev> Delay<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<DelayOperation, Ev>) -> Self {
        Self { context }
    }

    pub fn start(&self, millis: usize, event: Ev)
    where
        Ev: Send,
    {
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
