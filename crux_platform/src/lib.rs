//! TODO mod docs

use crux_core::{capability::CapabilityContext, Capability};
use serde::{Deserialize, Serialize};

#[derive(serde::Serialize)]
pub struct PlatformEffect;

// TODO revisit this
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformResponse(pub String);

impl crux_core::Effect for PlatformEffect {
    type Response = PlatformResponse;
}

pub struct Platform<Ev> {
    context: CapabilityContext<PlatformEffect, Ev>,
}

impl<Ev> Platform<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<PlatformEffect, Ev>) -> Self {
        Self { context }
    }

    pub fn get<F>(&self, callback: F)
    where
        F: Fn(PlatformResponse) -> Ev + Send + Sync + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let response = ctx.effect(PlatformEffect).await;

            ctx.send_event(callback(response));
        });
    }
}

impl<Ef> Capability<Ef> for Platform<Ef> {
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
