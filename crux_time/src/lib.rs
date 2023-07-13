//! Current time access for Crux apps
//!
//! Current time (on a wall clock) is considered a side-effect (although if we were to get pedantic, it's
//! more of a side-cause) by Crux, and has to be obtained externally. This capability provides a simple
//! interface to do so.
//!
//! This is still work in progress and as such very basic. It returns time as an IS08601 string.
use crux_core::capability::{CapabilityContext, Operation};
use crux_macros::Capability;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeRequest;

// TODO revisit this
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeResponse(pub String);

impl Operation for TimeRequest {
    type Output = TimeResponse;
}

/// The Time capability API.
#[derive(Capability)]
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

    /// Request current time, which will be passed to the app as `TimeResponse`
    /// wrapped in the event produced by the `callback`.
    pub fn get<F>(&self, callback: F)
    where
        F: Fn(TimeResponse) -> Ev + Send + Sync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            async move {
                let response = context.request_from_shell(TimeRequest).await;

                context.update_app(callback(response));
            }
        });
    }
}
