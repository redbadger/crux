use std::{
    future::Future,
    marker::PhantomData,
    time::{Duration, SystemTime},
};

use crux_core::{Command, Request, command::RequestBuilder};
use facet::Facet;
use futures::{
    FutureExt,
    channel::oneshot::{self, Sender},
    select_biased,
};

use crate::{TimeRequest, TimeResponse, TimerId, get_timer_id};

/// Result of the timer run. Timers can either run to completion or be cleared early.
#[derive(Facet, Debug, PartialEq, Eq, Clone)]
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
    pub fn now() -> RequestBuilder<Effect, Event, impl Future<Output = SystemTime>> {
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
        system_time: SystemTime,
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
        duration: Duration,
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
#[derive(Facet, Debug, PartialEq, Eq, Clone)]
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crux_core::Request;

    use super::{Time, TimerOutcome};
    use crate::{TimeRequest, TimeResponse};

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
        let (cmd, handle) = Time::notify_after(Duration::from_secs(2));
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
        let (cmd, handle) = Time::notify_after(Duration::from_secs(2));
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
        let (cmd, handle) = Time::notify_after(Duration::from_secs(2));
        let mut cmd = cmd.then_send(Event::Elapsed);

        let effect: Effect = cmd.effects().next().expect("Expected an effect!");

        drop(effect);
        assert!(!cmd.is_done());

        drop(handle);
        assert!(cmd.is_done());
    }
}
