//! Current time access for Crux apps
//!
//! Current time (on a wall clock) is considered a side-effect (although if we were to get pedantic, it's
//! more of a side-cause) by Crux, and has to be obtained externally. This capability provides a simple
//! interface to do so.

pub mod duration;
pub mod error;
pub mod instant;

pub use duration::Duration;
pub use error::TimeError;
pub use instant::Instant;

use serde::{Deserialize, Serialize};

use crux_core::capability::{CapabilityContext, Operation};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimeRequest {
    Now,
    NotifyAt(Instant),
    NotifyAfter(Duration),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimeResponse {
    Now(Instant),
    InstantArrived,
    DurationElapsed,
}

impl Operation for TimeRequest {
    type Output = TimeResponse;
}

/// The Time capability API
///
/// This capability provides access to the current time and allows the app to ask for
/// notifications when a specific instant has arrived or a duration has elapsed.
#[derive(crux_core::macros::Capability)]
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

    /// Request current time, which will be passed to the app as a [`TimeResponse`] containing an [`Instant`]
    /// wrapped in the event produced by the `callback`.
    pub fn now<F>(&self, callback: F)
    where
        F: Fn(TimeResponse) -> Ev + Send + Sync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            let this = self.clone();

            async move {
                context.update_app(callback(this.now_async().await));
            }
        });
    }

    /// Request current time, which will be passed to the app as a [`TimeResponse`] containing an [`Instant`]
    /// This is an async call to use with [`crux_core::compose::Compose`].
    pub async fn now_async(&self) -> TimeResponse {
        self.context.request_from_shell(TimeRequest::Now).await
    }

    /// Ask to receive a notification when the specified [`Instant`] has arrived.
    pub fn notify_at<F>(&self, instant: Instant, callback: F)
    where
        F: Fn(TimeResponse) -> Ev + Send + Sync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            let this = self.clone();

            async move {
                context.update_app(callback(this.notify_at_async(instant).await));
            }
        });
    }

    /// Ask to receive a notification when the specified [`Instant`] has arrived.
    /// This is an async call to use with [`crux_core::compose::Compose`].
    pub async fn notify_at_async(&self, instant: Instant) -> TimeResponse {
        self.context
            .request_from_shell(TimeRequest::NotifyAt(instant))
            .await
    }

    /// Ask to receive a notification when the specified duration has elapsed.
    pub fn notify_after<F>(&self, duration: Duration, callback: F)
    where
        F: Fn(TimeResponse) -> Ev + Send + Sync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            let this = self.clone();

            async move {
                context.update_app(callback(this.notify_after_async(duration).await));
            }
        });
    }

    /// Ask to receive a notification when the specified duration has elapsed.
    /// This is an async call to use with [`crux_core::compose::Compose`].
    pub async fn notify_after_async(&self, duration: Duration) -> TimeResponse {
        self.context
            .request_from_shell(TimeRequest::NotifyAfter(duration))
            .await
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_serializing_the_request_types_as_json() {
        let now = TimeRequest::Now;

        let serialized = serde_json::to_string(&now).unwrap();
        assert_eq!(&serialized, "\"now\"");

        let deserialized: TimeRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(now, deserialized);

        let now = TimeRequest::NotifyAt(Instant::new(1, 2).expect("valid instant"));

        let serialized = serde_json::to_string(&now).unwrap();
        assert_eq!(&serialized, r#"{"notifyAt":{"seconds":1,"nanos":2}}"#);

        let deserialized: TimeRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(now, deserialized);

        let now = TimeRequest::NotifyAfter(Duration::from_secs(1).expect("valid duration"));

        let serialized = serde_json::to_string(&now).unwrap();
        assert_eq!(&serialized, r#"{"notifyAfter":{"nanos":1000000000}}"#);

        let deserialized: TimeRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(now, deserialized);
    }

    #[test]
    fn test_serializing_the_response_types_as_json() {
        let now = TimeResponse::Now(Instant::new(1, 2).expect("valid instant"));

        let serialized = serde_json::to_string(&now).unwrap();
        assert_eq!(&serialized, r#"{"now":{"seconds":1,"nanos":2}}"#);

        let deserialized: TimeResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(now, deserialized);

        let now = TimeResponse::DurationElapsed;

        let serialized = serde_json::to_string(&now).unwrap();
        assert_eq!(&serialized, r#""durationElapsed""#);

        let deserialized: TimeResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(now, deserialized);

        let now = TimeResponse::InstantArrived;

        let serialized = serde_json::to_string(&now).unwrap();
        assert_eq!(&serialized, r#""instantArrived""#);

        let deserialized: TimeResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(now, deserialized);
    }
}
