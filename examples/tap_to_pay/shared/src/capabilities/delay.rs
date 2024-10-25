use crux_core::capability::{CapabilityContext, Operation};
use crux_core::Command;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum DelayOperation {
    Start { millis: usize },
}

impl Operation for DelayOperation {
    type Output = ();
}

#[derive(crux_core::macros::Capability)]
pub struct Delay {
    context: CapabilityContext<DelayOperation>,
}

impl Delay {
    pub fn new(context: CapabilityContext<DelayOperation>) -> Self {
        Self { context }
    }

    pub fn start<Ev: 'static + Send>(&self, millis: usize, event: Ev) -> Command<Ev> {
        let context = self.context.clone();
        Command::effect(async move {
            context
                .request_from_shell(DelayOperation::Start { millis })
                .await;
            Command::event(event)
        })
    }
}
