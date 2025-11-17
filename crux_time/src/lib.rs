//! Current time access for Crux apps
//!
//! Current time (on a wall clock) is considered a side-effect (although if we were to get pedantic, it's
//! more of a side-cause) by Crux, and has to be obtained externally. This capability provides a simple
//! interface to do so.

pub mod command;
pub mod protocol;

use std::{
    collections::HashSet,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    task::Poll,
    time,
};

use crux_core::{Command, Request, command::RequestBuilder};
use futures::{
    FutureExt,
    channel::oneshot::{self, Sender},
    select_biased,
};

pub use protocol::*;

/// Result of the timer run. Timers can either run to completion or be cleared early.
#[derive(Debug, PartialEq, Eq, Clone)]
#[repr(C)]
pub enum TimerOutcome {
    /// Timer completed successfully.
    Completed(CompletedTimerHandle),
    /// Timer was cleared early.
    Cleared,
}

/// Time capability API.
///
/// This capability provides access to the current time and allows the app to ask for
/// notifications when a specific instant has arrived or a duration has elapsed.
///
/// The capability also supports cancellation from the core side, using the [`TimerHandle`]
/// returned by [`notify_at`](Time::notify_at) and [`notify_after`](Time::notify_after).
pub struct Time<Effect, Event> {
    // Allow impl level trait bounds to avoid repetition
    effect: PhantomData<Effect>,
    event: PhantomData<Event>,
}

impl<Effect, Event> Time<Effect, Event>
where
    Effect: Send + From<Request<TimeRequest>> + 'static,
    Event: Send + 'static,
{
    /// Ask for the current wall-clock time.
    ///
    /// # Panics
    /// Panics if the response is not `TimeResponse::Now`.
    #[must_use]
    pub fn now() -> RequestBuilder<Effect, Event, impl Future<Output = time::SystemTime>> {
        Command::request_from_shell(TimeRequest::Now).map(|r| {
            let TimeResponse::Now { instant } = r else {
                panic!("Incorrect response received for TimeRequest::Now")
            };

            instant.into()
        })
    }

    /// Ask to receive a notification when the specified
    /// [`SystemTime`] has arrived. Returns the `RequestBuilder` alongside a [`TimerHandle`],
    /// which can be stored and used to clear the timer.
    ///
    /// # Panics
    /// Panics if the response is not `TimeResponse::InstantArrived`
    /// or if the timer ID is not the same as the one used to create the handle.
    #[must_use]
    pub fn notify_at(
        system_time: time::SystemTime,
    ) -> (
        RequestBuilder<Effect, Event, impl Future<Output = TimerOutcome>>,
        TimerHandle,
    ) {
        let timer_id = get_timer_id();
        let (sender, mut receiver) = oneshot::channel();

        let handle = TimerHandle {
            timer_id,
            abort: sender,
        };

        let completed_handle = CompletedTimerHandle { timer_id };

        // The `panic`s in the body of the builder would be `unreachable`s in Rust,
        // but since the shell is involved we can't check for them statically. Either way,
        // they are a developer error and suggest something quite wrong with the time implementation
        // in the shell.
        let builder = RequestBuilder::new(move |ctx| {
            async move {
                if let Ok(Some(cleared_id)) = receiver.try_recv() {
                    if cleared_id == timer_id {
                        return TimerOutcome::Cleared;
                    }
                }

                select_biased! {
                    response = ctx.request_from_shell(
                        TimeRequest::NotifyAt {
                            id: timer_id,
                            instant: system_time.into()
                        }
                    ).fuse() =>  {
                        let TimeResponse::InstantArrived { id } = response else {
                            panic!("Unexpected response to TimeRequest::NotifyAt");
                        };

                        assert!(id == timer_id, "InstantArrived with unexpected timer ID");

                        TimerOutcome::Completed(completed_handle)
                    },
                    cleared = receiver => {
                        // The Err variant would mean the sender was dropped,
                        // but `receiver` is a fused future,
                        // which signals `is_terminated` true in that case,
                        // so this branch of the select will
                        // never run for the Err case
                        let cleared_id = cleared.unwrap();

                        // Follow up by asking the shell to clear the timer
                        let TimeResponse::Cleared { id } = ctx.request_from_shell(TimeRequest::Clear { id: cleared_id }).await else {
                            panic!("Unexpected response to TimeRequest::Clear");
                        };

                        assert!(id == cleared_id, "Cleared with unexpected timer ID");

                        TimerOutcome::Cleared
                    }
                }
            }
        });

        (builder, handle)
    }

    /// Ask to receive a notification after the specified
    /// [`Duration`] has elapsed. Returns the `RequestBuilder` alongside a [`TimerHandle`],
    /// which can be stored and used to clear the timer.
    ///
    /// # Panics
    /// Panics if the response is not `TimeResponse::DurationElapsed`
    /// or if the timer ID is not the same as the one used to create the handle.
    #[must_use]
    pub fn notify_after(
        duration: time::Duration,
    ) -> (
        RequestBuilder<Effect, Event, impl Future<Output = TimerOutcome>>,
        TimerHandle,
    ) {
        let timer_id = get_timer_id();
        let (sender, mut receiver) = oneshot::channel();

        let handle = TimerHandle {
            timer_id,
            abort: sender,
        };

        let completed_handle = CompletedTimerHandle { timer_id };

        let builder = RequestBuilder::new(move |ctx| async move {
            if let Ok(Some(cleared_id)) = receiver.try_recv() {
                if cleared_id == timer_id {
                    return TimerOutcome::Cleared;
                }
            }

            select_biased! {
                response = ctx.request_from_shell(
                    TimeRequest::NotifyAfter {
                        id: timer_id,
                        duration: duration.into()
                    }
                ).fuse() => {
                    let TimeResponse::DurationElapsed { id } = response else {
                        panic!("Unexpected response to TimeRequest::NotifyAt");
                    };

                    assert!(id == timer_id, "InstantArrived with unexpected timer ID");

                    TimerOutcome::Completed(completed_handle)
                }
                cleared = receiver => {
                    // The Err variant would mean the sender was dropped,
                    // but `receiver` is a fused future,
                    // which signals `is_terminated` true in that case,
                    // so this branch of the select will
                    // never run for the Err case
                    let cleared_id = cleared.unwrap();
                    if cleared_id != timer_id {
                        unreachable!("Cleared with the wrong ID");
                    }

                    // Follow up by asking the shell to clear the timer
                    let TimeResponse::Cleared { id } = ctx.request_from_shell(TimeRequest::Clear { id: cleared_id }).await else {
                        panic!("Unexpected response to TimeRequest::Clear");
                    };

                    assert!(id == cleared_id, "Cleared resolved with unexpected timer ID");

                    TimerOutcome::Cleared
                }
            }
        });

        (builder, handle)
    }
}

/// A handle to a requested timer. Allows the timer to be cleared. The handle is safe to drop,
/// in which case the original timer is no longer abortable
#[derive(Debug)]
pub struct TimerHandle {
    timer_id: TimerId,
    abort: Sender<TimerId>,
}

impl TimerHandle {
    /// Clear the associated timer request.
    /// The shell will be notified that the timer has been cleared
    /// with `TimeRequest::Clear { id }`,
    /// so it can clean up associated resources.
    /// The original task will resolve
    /// with `TimeResponse::Cleared { id }`.
    pub fn clear(self) {
        let _ = self.abort.send(self.timer_id);
    }
}

/// Equivalent of [`TimerHandle`] for timers which completed (i.e. specified time is in the past).
///
/// `CompletedTimerHandle` can no longer be cleared, but can be compared with a
/// previously stored `TimerHandle`, if the app uses several timers at the same time.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CompletedTimerHandle {
    timer_id: TimerId,
}

impl Eq for TimerHandle {}

impl PartialEq for TimerHandle {
    fn eq(&self, other: &Self) -> bool {
        self.timer_id == other.timer_id
    }
}

impl PartialEq<CompletedTimerHandle> for TimerHandle {
    fn eq(&self, other: &CompletedTimerHandle) -> bool {
        self.timer_id == other.timer_id
    }
}

impl PartialEq<TimerHandle> for CompletedTimerHandle {
    fn eq(&self, other: &TimerHandle) -> bool {
        self.timer_id == other.timer_id
    }
}

impl From<TimerHandle> for CompletedTimerHandle {
    fn from(value: TimerHandle) -> Self {
        Self {
            timer_id: value.timer_id,
        }
    }
}

fn get_timer_id() -> TimerId {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    TimerId(COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub struct TimerFuture<F>
where
    F: Future<Output = TimeResponse> + Unpin,
{
    timer_id: TimerId,
    is_cleared: bool,
    future: F,
}

impl<F> Future for TimerFuture<F>
where
    F: Future<Output = TimeResponse> + Unpin,
{
    type Output = TimeResponse;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if self.is_cleared {
            // short-circuit return
            return Poll::Ready(TimeResponse::Cleared { id: self.timer_id });
        }
        // see if the timer has been cleared
        let timer_is_cleared = {
            let mut lock = CLEARED_TIMER_IDS.lock().unwrap();
            lock.remove(&self.timer_id)
        };
        let this = self.get_mut();
        this.is_cleared = timer_is_cleared;
        if timer_is_cleared {
            // if the timer has been cleared, immediately return 'Ready' without
            // waiting for the timer to elapse
            Poll::Ready(TimeResponse::Cleared { id: this.timer_id })
        } else {
            // otherwise, defer to the inner future
            Pin::new(&mut this.future).poll(cx)
        }
    }
}

// Global HashSet containing the ids of timers which have been _cleared_
// but the whose futures have _not since been polled_. When the future is next
// polled, the timer id is evicted from this set and the timer is 'poisoned'
// so as to return immediately without waiting on the shell.
static CLEARED_TIMER_IDS: LazyLock<Mutex<HashSet<TimerId>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

#[cfg(test)]
mod test {
    use super::*;

    use crux_core::Request;

    use super::{Time, TimerOutcome};
    use crate::Instant;
    use crate::protocol::duration::Duration;
    use crate::{TimeRequest, TimeResponse};

    #[test]
    fn test_serializing_the_request_types_as_json() {
        let now = TimeRequest::Now;

        let serialized = serde_json::to_string(&now).unwrap();
        assert_eq!(&serialized, "\"now\"");

        let deserialized: TimeRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(now, deserialized);

        let now = TimeRequest::NotifyAt {
            id: TimerId(1),
            instant: Instant::new(1, 2),
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
            duration: Duration::from_secs(1),
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
        let now = TimeResponse::Now {
            instant: Instant::new(1, 2),
        };

        let serialized = serde_json::to_string(&now).unwrap();
        assert_eq!(
            &serialized,
            r#"{"now":{"instant":{"seconds":1,"nanos":2}}}"#
        );

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

    enum Effect {
        Time(Request<TimeRequest>),
    }

    impl From<Request<TimeRequest>> for Effect {
        fn from(value: Request<TimeRequest>) -> Self {
            Self::Time(value)
        }
    }

    #[derive(Debug, PartialEq)]
    enum Event {
        Elapsed(TimerOutcome),
    }

    #[test]
    fn timer_can_be_cleared() {
        let (cmd, handle) = Time::notify_after(core::time::Duration::from_secs(2));
        let mut cmd = cmd.then_send(Event::Elapsed);

        let effect = cmd.effects().next();

        assert!(cmd.events().next().is_none());

        let Some(Effect::Time(_request)) = effect else {
            panic!("should get an effect");
        };

        handle.clear();

        let effect = cmd.effects().next();
        assert!(cmd.events().next().is_none());

        let Some(Effect::Time(mut request)) = effect else {
            panic!("should get an effect");
        };

        let TimeRequest::Clear { id } = request.operation else {
            panic!("expected a Clear request");
        };

        request
            .resolve(TimeResponse::Cleared { id })
            .expect("should resolve");

        let event = cmd.events().next();

        assert!(matches!(event, Some(Event::Elapsed(TimerOutcome::Cleared))));
    }

    #[test]
    fn dropping_a_timer_handle_does_not_clear_the_request() {
        let (cmd, handle) = Time::notify_after(core::time::Duration::from_secs(2));
        drop(handle);

        let mut cmd = cmd.then_send(Event::Elapsed);
        let effect = cmd.effects().next();

        assert!(cmd.events().next().is_none());

        let Some(Effect::Time(mut request)) = effect else {
            panic!("should get an effect");
        };

        let TimeRequest::NotifyAfter { id, .. } = request.operation else {
            panic!("Expected a NotifyAfter");
        };

        request
            .resolve(TimeResponse::DurationElapsed { id })
            .expect("should resolve");

        let event = cmd.events().next();

        assert!(matches!(
            event,
            Some(Event::Elapsed(TimerOutcome::Completed(_)))
        ));
    }

    #[test]
    fn dropping_a_timer_request_while_holding_a_handle_and_polling() {
        let (cmd, handle) = Time::notify_after(core::time::Duration::from_secs(2));
        let mut cmd = cmd.then_send(Event::Elapsed);

        let effect: Effect = cmd.effects().next().expect("Expected an effect!");

        drop(effect);
        assert!(!cmd.is_done());

        drop(handle);
        assert!(cmd.is_done());
    }
}
