//! TODO mod docs

use crux_core::{
    capability::{CapabilityContext, Operation},
    Capability,
};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Serialize)]
pub struct TimeRequest;

// TODO revisit this
#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeResponse(pub String);

impl Operation for TimeRequest {
    type Output = TimeResponse;
}

pub struct Time<Ev> {
    context: CapabilityContext<TimeRequest, Ev>,
}

impl<Ev> Time<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<TimeRequest, Ev>) -> Self {
        Self { context }
    }

    pub fn get<F>(&self, callback: F)
    where
        F: Fn(TimeResponse) -> Ev + Send + Sync + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let response = ctx.request_effect(TimeRequest).await;

            ctx.dispatch(callback(response));
        });
    }
}

impl<Ef> Capability<Ef> for Time<Ef> {
    type MappedSelf<MappedEv> = Time<MappedEv>;

    fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
    where
        F: Fn(NewEvent) -> Ef + Send + Sync + Copy + 'static,
        Ef: 'static,
        NewEvent: 'static,
    {
        Time::new(self.context.map_event(f))
    }
}
