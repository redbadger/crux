//! TODO mod docs

use crux_core::{
    capability::{CapabilityContext, Operation},
    Capability,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformRequest;

// TODO revisit this
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformResponse(pub String);

impl Operation for PlatformRequest {
    type Output = PlatformResponse;
}

pub struct Platform<Ev> {
    context: CapabilityContext<PlatformRequest, Ev>,
}

impl<Ev> Platform<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<PlatformRequest, Ev>) -> Self {
        Self { context }
    }

    pub fn get<F>(&self, callback: F)
    where
        F: Fn(PlatformResponse) -> Ev + Send + Sync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            async move {
                let response = context.request_from_shell(PlatformRequest).await;

                context.update_app(callback(response));
            }
        });
    }
}

impl<Ef> Capability<Ef> for Platform<Ef> {
    type Operation = PlatformRequest;
    type MappedSelf<MappedEv> = Platform<MappedEv>;

    fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
    where
        F: Fn(NewEvent) -> Ef + Send + Sync + Copy + 'static,
        Ef: 'static,
        NewEvent: 'static,
    {
        Platform::new(self.context.map_event(f))
    }
}
