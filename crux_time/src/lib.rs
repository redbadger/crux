//! Current time access for Crux apps
//!
//! Current time (on a wall clock) is considered a side-effect (although if we were to get pedantic, it's
//! more of a side-cause) by Crux, and has to be obtained externally. This capability provides a simple
//! interface to do so.
//!
//! This is still work in progress and as such very basic.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crux_core::capability::{CapabilityContext, Operation};

/// Error type for time operations
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Error)]
pub enum TimeError {
    #[error("invalid time")]
    InvalidTime,
}

/// Represents a point in time:
///
/// - seconds: number of seconds since the Unix epoch (1970-01-01T00:00:00Z)
/// - sub_nanos: number of nanoseconds since the last second
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instant {
    pub seconds: u64,
    pub sub_nanos: u32,
}

impl TryFrom<Instant> for DateTime<Utc> {
    type Error = TimeError;

    fn try_from(time: Instant) -> Result<Self, Self::Error> {
        let seconds = i64::try_from(time.seconds).map_err(|_| TimeError::InvalidTime)?;
        DateTime::<Utc>::from_timestamp(seconds, time.sub_nanos).ok_or(TimeError::InvalidTime)
    }
}

impl TryFrom<DateTime<Utc>> for Instant {
    type Error = TimeError;

    fn try_from(time: DateTime<Utc>) -> Result<Self, Self::Error> {
        let seconds = time.timestamp();
        let sub_nanos = time.timestamp_subsec_nanos();
        Ok(Instant {
            seconds: seconds.try_into().map_err(|_| TimeError::InvalidTime)?,
            sub_nanos,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeRequest {
    Now,
    SubscribeInstant(Instant),
    SubscribeDuration(u64),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
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
/// This capability provides access to the current time and allows the app to subscribe to
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
            async move {
                context.update_app(callback(context.request_from_shell(TimeRequest::Now).await));
            }
        });
    }

    /// Request current time, which will be passed to the app as a [`TimeResponse`] containing an [`Instant`]
    /// This is an async call to use with [`crux_core::compose::Compose`].
    pub async fn now_async(&self) -> TimeResponse {
        self.context.request_from_shell(TimeRequest::Now).await
    }

    /// Subscribe to receive a notification when the specified [`Instant`] has arrived.
    pub fn subscribe_instant<F>(&self, instant: Instant, callback: F)
    where
        F: Fn(TimeResponse) -> Ev + Send + Sync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            async move {
                context.update_app(callback(
                    context
                        .request_from_shell(TimeRequest::SubscribeInstant(instant))
                        .await,
                ));
            }
        });
    }

    /// Subscribe to receive a notification when the specified [`Instant`] has arrived.
    /// This is an async call to use with [`crux_core::compose::Compose`].
    pub async fn subscribe_instant_async(&self, instant: Instant) -> TimeResponse {
        self.context
            .request_from_shell(TimeRequest::SubscribeInstant(instant))
            .await
    }

    /// Subscribe to receive a notification when the specified duration has elapsed.
    pub fn subscribe_duration<F>(&self, duration: u64, callback: F)
    where
        F: Fn(TimeResponse) -> Ev + Send + Sync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            async move {
                context.update_app(callback(
                    context
                        .request_from_shell(TimeRequest::SubscribeDuration(duration))
                        .await,
                ));
            }
        });
    }

    /// Subscribe to receive a notification when the specified duration has elapsed.
    /// This is an async call to use with [`crux_core::compose::Compose`].
    pub async fn subscribe_duration_async(&self, duration: u64) -> TimeResponse {
        self.context
            .request_from_shell(TimeRequest::SubscribeDuration(duration))
            .await
    }
}
