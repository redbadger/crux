//! The time capability, one operation type at a time.
//!
//! This module holds [`Time`], the per-operation time capability. The type of
//! the same name at the crate root — [`crate::Time`] — is the enum-based API
//! that this one replaces.
//!
//! The two coexist in this release, so an app can move one call at a time
//! rather than rewriting every timer at once. A later breaking release removes
//! the root type and re-exports this one in its place: `crux_time::clock::Time`
//! will go on meaning exactly what it means today, and `crux_time::Time` will
//! come to mean this type.

use std::{future::Future, marker::PhantomData, time};

use crux_core::{Command, Request, command::RequestBuilder};
use futures::{FutureExt, channel::oneshot, select_biased};

use crate::{CompletedTimerHandle, TimerHandle, TimerOutcome, get_timer_id, operation};

/// Time capability API, one operation type at a time.
///
/// The same API as the enum-based [`Time`](crate::Time) at the crate root,
/// built from the per-operation types in [`operation`]: each method sends its
/// own operation type, whose single output type the shell cannot get wrong.
///
/// Cancellation works exactly as it does with the enum-based API: the
/// [`TimerHandle`] returned by [`notify_at`](Time::notify_at) and
/// [`notify_after`](Time::notify_after) clears the timer, which asks the shell
/// to release it with an [`operation::Clear`] request and resolves the timer's
/// future with [`TimerOutcome::Cleared`] once the shell has answered.
///
/// The bounds are per method, so an app's `Effect` only has to carry the
/// operations it actually uses:
///
/// ```
/// # use crux_core::{Command, macros::effect};
/// use crux_time::{TimerOutcome, clock::Time, operation};
///
/// #[effect]
/// enum Effect {
///     Now(operation::Now),
///     NotifyAfter(operation::NotifyAfter),
///     Clear(operation::Clear),
/// }
///
/// # enum Event { Now(std::time::SystemTime), Elapsed(TimerOutcome) }
/// let now: Command<Effect, Event> = Time::now().then_send(Event::Now);
///
/// let (builder, _handle) = Time::notify_after(std::time::Duration::from_secs(1));
/// let timer: Command<Effect, Event> = builder.then_send(Event::Elapsed);
/// ```
pub struct Time<Effect, Event> {
    // Allow impl level trait bounds to avoid repetition
    effect: PhantomData<Effect>,
    event: PhantomData<Event>,
}

impl<Effect, Event> Time<Effect, Event>
where
    Effect: Send + 'static,
    Event: Send + 'static,
{
    /// Ask for the current wall-clock time.
    #[must_use]
    pub fn now() -> RequestBuilder<Effect, Event, impl Future<Output = time::SystemTime>>
    where
        Effect: From<Request<operation::Now>>,
    {
        Command::request_from_shell(operation::Now).map(Into::into)
    }

    /// Ask to receive a notification when the specified
    /// [`SystemTime`](std::time::SystemTime) has arrived. Returns the `RequestBuilder`
    /// alongside a [`TimerHandle`], which can be stored and used to clear the timer.
    ///
    /// # Panics
    /// Panics if the timer ID the shell answers with is not the one it was asked about.
    #[must_use]
    pub fn notify_at(
        system_time: time::SystemTime,
    ) -> (
        RequestBuilder<Effect, Event, impl Future<Output = TimerOutcome>>,
        TimerHandle,
    )
    where
        Effect: From<Request<operation::NotifyAt>> + From<Request<operation::Clear>>,
    {
        let timer_id = get_timer_id();
        let (sender, mut receiver) = oneshot::channel();

        let handle = TimerHandle {
            timer_id,
            abort: sender,
        };

        let completed_handle = CompletedTimerHandle { timer_id };

        // The `assert`s in the body of the builder would be `unreachable`s in Rust,
        // but since the shell is involved we can't check for them statically. Either way,
        // they are a developer error and suggest something quite wrong with the time
        // implementation in the shell.
        let builder = RequestBuilder::new(move |ctx| async move {
            if let Ok(Some(cleared_id)) = receiver.try_recv()
                && cleared_id == timer_id
            {
                return TimerOutcome::Cleared;
            }

            select_biased! {
                id = ctx.request_from_shell(
                    operation::NotifyAt {
                        id: timer_id,
                        instant: system_time.into(),
                    }
                ).fuse() => {
                    assert_eq!(id, timer_id, "NotifyAt resolved with an unexpected timer ID");

                    TimerOutcome::Completed(completed_handle)
                },
                cleared = receiver => {
                    // The Err variant would mean the sender was dropped,
                    // but `receiver` is a fused future,
                    // which signals `is_terminated` true in that case,
                    // so this branch of the select will
                    // never run for the Err case
                    let cleared_id = cleared.unwrap();

                    assert_eq!(cleared_id, timer_id, "cleared with an unexpected timer ID");

                    // Ask the shell to clear the timer, so it can clean up, and
                    // wait for it to say it has.
                    let id = ctx.request_from_shell(operation::Clear { id: cleared_id }).await;

                    assert_eq!(id, cleared_id, "Clear resolved with an unexpected timer ID");

                    TimerOutcome::Cleared
                }
            }
        });

        (builder, handle)
    }

    /// Ask to receive a notification after the specified
    /// [`Duration`](std::time::Duration) has elapsed. Returns the `RequestBuilder`
    /// alongside a [`TimerHandle`], which can be stored and used to clear the timer.
    ///
    /// # Panics
    /// Panics if the timer ID the shell answers with is not the one it was asked about.
    #[must_use]
    pub fn notify_after(
        duration: time::Duration,
    ) -> (
        RequestBuilder<Effect, Event, impl Future<Output = TimerOutcome>>,
        TimerHandle,
    )
    where
        Effect: From<Request<operation::NotifyAfter>> + From<Request<operation::Clear>>,
    {
        let timer_id = get_timer_id();
        let (sender, mut receiver) = oneshot::channel();

        let handle = TimerHandle {
            timer_id,
            abort: sender,
        };

        let completed_handle = CompletedTimerHandle { timer_id };

        let builder = RequestBuilder::new(move |ctx| async move {
            if let Ok(Some(cleared_id)) = receiver.try_recv()
                && cleared_id == timer_id
            {
                return TimerOutcome::Cleared;
            }

            select_biased! {
                id = ctx.request_from_shell(
                    operation::NotifyAfter {
                        id: timer_id,
                        duration: duration.into(),
                    }
                ).fuse() => {
                    assert_eq!(id, timer_id, "NotifyAfter resolved with an unexpected timer ID");

                    TimerOutcome::Completed(completed_handle)
                },
                cleared = receiver => {
                    // The Err variant would mean the sender was dropped,
                    // but `receiver` is a fused future,
                    // which signals `is_terminated` true in that case,
                    // so this branch of the select will
                    // never run for the Err case
                    let cleared_id = cleared.unwrap();

                    assert_eq!(cleared_id, timer_id, "cleared with an unexpected timer ID");

                    // Ask the shell to clear the timer, so it can clean up, and
                    // wait for it to say it has.
                    let id = ctx.request_from_shell(operation::Clear { id: cleared_id }).await;

                    assert_eq!(id, cleared_id, "Clear resolved with an unexpected timer ID");

                    TimerOutcome::Cleared
                }
            }
        });

        (builder, handle)
    }
}
