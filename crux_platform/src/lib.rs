//! TODO mod docs

use crux_core::capability::{CapabilityContext, Operation};
use crux_core::macros::Capability;
use crux_core::Command;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformRequest;

// TODO revisit this
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformResponse(pub String);

impl Operation for PlatformRequest {
    type Output = PlatformResponse;
}

#[derive(Capability, Clone)]
pub struct Platform {
    context: CapabilityContext<PlatformRequest>,
}

impl Platform {
    pub fn new(context: CapabilityContext<PlatformRequest>) -> Self {
        Self { context }
    }

    pub fn get<F, Ev>(&self, callback: F) -> Command<Ev>
    where
        F: FnOnce(PlatformResponse) -> Ev + Send + Sync + 'static,
    {
        let context = self.context.clone();
        Command::effect(async move {
            let response = context.request_from_shell(PlatformRequest).await;
            Command::event(callback(response))
        })
    }
}
