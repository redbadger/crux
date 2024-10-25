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

use crux_core::{
    capability::{CapabilityContext, Operation},
    Command,
};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimeRequest {
    Now,
    NotifyAt { id: TimerId, instant: Instant },
    NotifyAfter { id: TimerId, duration: Duration },
    Clear { id: TimerId },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimerId(pub usize);

fn get_timer_id() -> TimerId {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    TimerId(COUNTER.fetch_add(1, Ordering::Relaxed))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimeResponse {
    Now(Instant),
    InstantArrived { id: TimerId },
    DurationElapsed { id: TimerId },
    Cleared { id: TimerId },
}

impl Operation for TimeRequest {
    type Output = TimeResponse;
}

/// The Time capability API
///
/// This capability provides access to the current time and allows the app to ask for
/// notifications when a specific instant has arrived or a duration has elapsed.
pub struct Time {
    context: CapabilityContext<TimeRequest>,
}

impl crux_core::Capability for Time {
    type Operation = TimeRequest;

    #[cfg(feature = "typegen")]
    fn register_types(generator: &mut crux_core::typegen::TypeGen) -> crux_core::typegen::Result {
        generator.register_type::<Instant>()?;
        generator.register_type::<Duration>()?;
        generator.register_type::<Self::Operation>()?;
        generator.register_type::<<Self::Operation as Operation>::Output>()?;
        Ok(())
    }
}

impl Clone for Time {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
        }
    }
}

impl Time {
    pub fn new(context: CapabilityContext<TimeRequest>) -> Self {
        Self { context }
    }

    /// Request current time, which will be passed to the app as a [`TimeResponse`] containing an [`Instant`]
    /// wrapped in the event produced by the `callback`.
    pub fn now<F, Ev>(&self, callback: F) -> Command<Ev>
    where
        F: FnOnce(TimeResponse) -> Ev + Send + Sync + 'static,
    {
        let this = self.clone();
        Command::effect(async move { Command::event(callback(this.now_async().await)) })
    }

    /// Request current time, which will be passed to the app as a [`TimeResponse`] containing an [`Instant`]
    /// This is an async call to use with [`crux_core::compose::Compose`].
    pub async fn now_async(&self) -> TimeResponse {
        self.context.request_from_shell(TimeRequest::Now).await
    }

    /// Ask to receive a notification when the specified [`Instant`] has arrived.
    pub fn notify_at<F, Ev>(&self, instant: Instant, callback: F) -> (Command<Ev>, TimerId)
    where
        F: FnOnce(TimeResponse) -> Ev + Send + Sync + 'static,
    {
        let tid = get_timer_id();
        let this = self.clone();

        let cmd = Command::effect(async move {
            Command::event(callback(this.notify_at_async(tid, instant).await))
        });

        (cmd, tid)
    }

    /// Ask to receive a notification when the specified [`Instant`] has arrived.
    pub async fn notify_at_async(&self, id: TimerId, instant: Instant) -> TimeResponse {
        self.context
            .request_from_shell(TimeRequest::NotifyAt { id, instant })
            .await
    }

    /// Ask to receive a notification when the specified duration has elapsed.
    pub fn notify_after<F, Ev>(&self, duration: Duration, callback: F) -> (Command<Ev>, TimerId)
    where
        F: FnOnce(TimeResponse) -> Ev + Send + Sync + 'static,
    {
        let tid = get_timer_id();
        let this = self.clone();
        let cmd = Command::effect(async move {
            Command::event(callback(this.notify_after_async(tid, duration).await))
        });

        (cmd, tid)
    }

    /// Ask to receive a notification when the specified duration has elapsed.
    /// This is an async call to use with [`crux_core::compose::Compose`].
    pub async fn notify_after_async(&self, id: TimerId, duration: Duration) -> TimeResponse {
        self.context
            .request_from_shell(TimeRequest::NotifyAfter { id, duration })
            .await
    }

    pub fn clear<Ev>(&self, id: TimerId) -> Command<Ev> {
        let ctx = self.context.clone();
        Command::effect(async move {
            ctx.notify_shell(TimeRequest::Clear { id }).await;
            Command::none()
        })
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

        let now = TimeRequest::NotifyAt {
            id: TimerId(1),
            instant: Instant::new(1, 2).expect("valid instant"),
        };

        let serialized = serde_json::to_string(&now).unwrap();
        assert_eq!(
            &serialized,
            r#"{"notifyAt":{"id":1,"instant":{"seconds":1,"nanos":2}}}"#
        );

        let deserialized: TimeRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(now, deserialized);

        let now = TimeRequest::NotifyAfter {
            id: TimerId(2),
            duration: Duration::from_secs(1).expect("valid duration"),
        };

        let serialized = serde_json::to_string(&now).unwrap();
        assert_eq!(
            &serialized,
            r#"{"notifyAfter":{"id":2,"duration":{"nanos":1000000000}}}"#
        );

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

        let now = TimeResponse::DurationElapsed { id: TimerId(1) };

        let serialized = serde_json::to_string(&now).unwrap();
        assert_eq!(&serialized, r#"{"durationElapsed":{"id":1}}"#);

        let deserialized: TimeResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(now, deserialized);

        let now = TimeResponse::InstantArrived { id: TimerId(2) };

        let serialized = serde_json::to_string(&now).unwrap();
        assert_eq!(&serialized, r#"{"instantArrived":{"id":2}}"#);

        let deserialized: TimeResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(now, deserialized);
    }
}
