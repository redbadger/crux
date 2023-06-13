use crux_core::capability::{CapabilityContext, Operation};
use crux_macros::Capability;
use futures::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum TimerOperation {
    Start { id: u64, millis: usize },
    Cancel { id: u64 },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TimerOutput {
    Created { id: u64 },
    Finished { id: u64 },
}

impl Operation for TimerOperation {
    type Output = TimerOutput;
}

#[derive(Capability)]
pub struct Timer<Event> {
    context: CapabilityContext<TimerOperation, Event>,
}

impl<Ev> Timer<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<TimerOperation, Ev>) -> Self {
        Self { context }
    }

    pub fn start<F>(&self, id: u64, millis: usize, make_event: F)
    where
        F: Fn(TimerOutput) -> Ev + Clone + Send + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();

            async move {
                let mut stream = context.stream_from_shell(TimerOperation::Start { id, millis });

                while let Some(output) = stream.next().await {
                    let make_event = make_event.clone();

                    context.update_app(make_event(output));
                }
            }
        })
    }

    pub fn cancel(&self, id: u64) {
        self.context.spawn({
            let context = self.context.clone();

            async move {
                context.notify_shell(TimerOperation::Cancel { id }).await;
            }
        })
    }
}
