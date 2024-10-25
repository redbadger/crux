use crux_core::capability::{CapabilityContext, Operation};
use crux_core::Command;
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

#[derive(crux_core::macros::Capability)]
pub struct Timer {
    context: CapabilityContext<TimerOperation>,
}

impl Timer {
    pub fn new(context: CapabilityContext<TimerOperation>) -> Self {
        Self { context }
    }

    pub fn start<F, Ev>(&self, id: u64, millis: usize, make_event: F) -> Command<Ev>
    where
        F: FnOnce(TimerOutput) -> Ev + Clone + Send + 'static,
    {
        let context = self.context.clone();
        let mut stream = context.stream_from_shell(TimerOperation::Start { id, millis });
        Command::stream(stream.map(move |message| {
            let make_event = make_event.clone();
            Command::event(make_event(message))
        }))
    }

    pub fn cancel<Event>(&self, id: u64) -> Command<Event> {
        let context = self.context.clone();
        Command::empty_effect(async move {
            context.notify_shell(TimerOperation::Cancel { id }).await;
        })
    }
}
