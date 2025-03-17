use std::{
    future::Future,
    marker::PhantomData,
    time::{Duration, SystemTime},
};

use crux_core::{command::RequestBuilder, Command, Request};
use futures::{
    channel::oneshot::{self, Sender},
    select, FutureExt,
};

use crate::{get_timer_id, TimeRequest, TimeResponse, TimerId};

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
    pub fn now() -> RequestBuilder<Effect, Event, impl Future<Output = TimeResponse>> {
        Command::request_from_shell(TimeRequest::Now)
    }

    /// Ask to receive a notification when the specified
    /// [`SystemTime`](std::time::SystemTime) has arrived.
    pub fn notify_at(
        system_time: SystemTime,
    ) -> (
        RequestBuilder<Effect, Event, impl Future<Output = TimeResponse>>,
        TimerHandle,
    ) {
        let id = get_timer_id();
        let (sender, mut receiver) = oneshot::channel();

        let builder = RequestBuilder::new(move |ctx| async move {
            select! {
                response = ctx.request_from_shell(
                    TimeRequest::NotifyAt {
                        id,
                        instant: system_time.into()
                    }
                ).fuse() =>  response,
                cleared = receiver => {
                    // The Err variant would mean the sender was dropped,
                    // but `receiver` is a fused future,
                    // which signals `is_terminated` true in that case,
                    // so this branch of the select will
                    // never run for the Err case

                    let id = cleared.unwrap();
                    ctx.request_from_shell(TimeRequest::Clear { id }).await
                }
            }
        });

        let handle = TimerHandle {
            timer_id: id,
            abort: sender,
        };

        (builder, handle)
    }

    /// Ask to receive a notification after the specified
    /// [`Duration`](std::time::Duration) has elapsed.
    pub fn notify_after(
        duration: Duration,
    ) -> (
        RequestBuilder<Effect, Event, impl Future<Output = TimeResponse>>,
        TimerHandle,
    ) {
        let id = get_timer_id();
        let (sender, mut receiver) = oneshot::channel();

        let builder = RequestBuilder::new(move |ctx| async move {
            select! {
                response = ctx.request_from_shell(
                    TimeRequest::NotifyAfter {
                        id,
                        duration: duration.into()
                    }
                ).fuse() => response,
                cleared = receiver => {
                    // The Err variant would mean the sender was dropped,
                    // but `receiver` is a fused future,
                    // which signals `is_terminated` true in that case,
                    // so this branch of the select will
                    // never run for the Err case
                    let id = cleared.unwrap();
                    ctx.request_from_shell(TimeRequest::Clear { id }).await
                }
            }
        });

        let handle = TimerHandle {
            timer_id: id,
            abort: sender,
        };

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

impl Eq for TimerHandle {}

impl PartialEq for TimerHandle {
    fn eq(&self, other: &Self) -> bool {
        self.timer_id == other.timer_id
    }
}

impl PartialEq<TimerId> for TimerHandle {
    fn eq(&self, other: &TimerId) -> bool {
        self.timer_id == *other
    }
}

impl PartialEq<TimerHandle> for TimerId {
    fn eq(&self, other: &TimerHandle) -> bool {
        *self == other.timer_id
    }
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crux_core::Request;

    use super::Time;
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
        Elapsed(TimeResponse),
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

        assert!(matches!(
            event,
            Some(Event::Elapsed(TimeResponse::Cleared { .. }))
        ));
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

        assert_eq!(
            event,
            Some(Event::Elapsed(TimeResponse::DurationElapsed { id }))
        );
    }
}
