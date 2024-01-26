//! Current time access for Crux apps
//!
//! Current time (on a wall clock) is considered a side-effect (although if we were to get pedantic, it's
//! more of a side-cause) by Crux, and has to be obtained externally. This capability provides a simple
//! interface to do so.
//!
//! This is still work in progress and as such very basic.
use chrono::{DateTime, Utc};
use crux_core::capability::{CapabilityContext, Operation};
use crux_macros::Capability;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeRequest;

impl Operation for TimeRequest {
    type Output = DateTime<Utc>;
}

/// The Time capability API. Uses the `chrono` crate's timezone aware representation
/// of the current date and time.
///
/// The time value serializes as an RFC3339 string across the FFI boundary.
#[derive(Capability)]
pub struct Time<Ev> {
    context: CapabilityContext<TimeRequest, Ev>,
}

impl<Ev> Clone for Time<Ev> {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
        }
    }
}

impl<Ev> Time<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<TimeRequest, Ev>) -> Self {
        Self { context }
    }

    /// Request current time, which will be passed to the app as `chrono::DateTime<Utc>`
    /// wrapped in the event produced by the `callback`.
    pub fn now<F>(&self, callback: F)
    where
        F: Fn(DateTime<Utc>) -> Ev + Send + Sync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            async move {
                let response = context.request_from_shell(TimeRequest).await;

                context.update_app(callback(response));
            }
        });
    }

    /// Request current time, which will be returned as `chrono::DateTime<Utc>`.
    /// This is an async call to use with [`crux_core::compose::Compose`].
    pub async fn now_async(&self) -> DateTime<Utc> {
        self.context.request_from_shell(TimeRequest).await
    }
}
