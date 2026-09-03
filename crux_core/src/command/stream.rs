#![allow(clippy::redundant_pub_crate)]
//! Stream support for [`Command`].
//!
//! Commands implement [`Stream`], yielding effect requests and events as they
//! become available. This is useful for testing, manual orchestration, and
//! advanced command composition.

use std::future::Future;
use std::ops::DerefMut as _;
use std::task::{Context, Poll};

use std::pin::Pin;

use futures::{Sink, Stream, StreamExt as _};

use thiserror::Error;

use super::{Command, CommandContext, CommandEvent};

impl<Effect, Event> Command<Effect, Event>
where
    Effect: Unpin + Send + 'static,
    Event: Unpin + Send + 'static,
{
    /// Borrow this command as a stream which preserves event origin.
    pub(crate) fn stream_with_origin(&mut self) -> CommandStreamWithOrigin<'_, Effect, Event> {
        CommandStreamWithOrigin { command: self }
    }

    /// Convert this command into a stream which preserves event origin.
    pub(crate) fn into_stream_with_origin(self) -> IntoCommandStreamWithOrigin<Effect, Event> {
        IntoCommandStreamWithOrigin { command: self }
    }

    /// Host this command inside another command context.
    ///
    /// Effects and structured command events emitted by `self` are forwarded
    /// into `context`. Event ownership is preserved, so events emitted by
    /// nested commands can receive their follow-up commands in the same
    /// command subtree.
    pub(crate) fn host(self, context: CommandContext<Effect, Event>) -> impl Future {
        self.into_stream_with_origin().host(context)
    }
}

/// An item emitted from a [`Command`] stream.
///
/// The public [`Stream`] implementation for [`Command`] uses `Event` as the
/// event payload. Crux runtime internals use the same enum with
/// [`CommandEvent`] as the event payload, preserving the event's originating
/// command context.
#[derive(Debug)]
pub enum CommandOutput<Effect, Event> {
    /// An effect request for the shell or an effect handler to process.
    Effect(Effect),
    /// An event emitted by a command.
    Event(Event),
}

impl<Effect, Event> Stream for Command<Effect, Event>
where
    Effect: Unpin + Send + 'static,
    Event: Unpin + Send + 'static,
{
    type Item = CommandOutput<Effect, Event>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.deref_mut()
            .stream_with_origin()
            .poll_next_unpin(cx)
            .map(|output| {
                output.map(|output| match output {
                    CommandOutput::Effect(effect) => CommandOutput::Effect(effect),
                    CommandOutput::Event(event) => CommandOutput::Event(event.into_event()),
                })
            })
    }
}

/// A borrowed stream over a command which preserves event origin.
///
/// Polling this stream advances the command executor until it settles, then
/// yields one queued event or effect. It registers the caller's waker
/// so nested commands can wake their host when new work is spawned through a
/// saved [`CommandContext`].
pub(crate) struct CommandStreamWithOrigin<'a, Effect, Event> {
    command: &'a mut Command<Effect, Event>,
}

impl<Effect, Event> Stream for CommandStreamWithOrigin<'_, Effect, Event>
where
    Effect: Unpin + Send + 'static,
    Event: Unpin + Send + 'static,
{
    type Item = CommandOutput<Effect, CommandEvent<Effect, Event>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.command.waker.register(cx.waker());

        // run_until_settled is idempotent
        self.command.run_until_settled();

        // Check events first to preserve the order in which items were emitted. This is because
        // sending events doesn't yield, and the next request/stream await point will be
        // reached in the same poll, so any follow up effects will _also_ be available
        if let Ok(event) = self.command.events.try_recv() {
            return Poll::Ready(Some(CommandOutput::Event(event)));
        }

        if let Ok(effect) = self.command.effects.try_recv() {
            return Poll::Ready(Some(CommandOutput::Effect(effect)));
        }

        if self.command.is_done() {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

/// An owned stream over a command which preserves event origin.
///
/// This is the consumed form used when commands are hosted or mapped.
pub(crate) struct IntoCommandStreamWithOrigin<Effect, Event> {
    command: Command<Effect, Event>,
}

impl<Effect, Event> Stream for IntoCommandStreamWithOrigin<Effect, Event>
where
    Effect: Unpin + Send + 'static,
    Event: Unpin + Send + 'static,
{
    type Item = CommandOutput<Effect, CommandEvent<Effect, Event>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        this.command.stream_with_origin().poll_next_unpin(cx)
    }
}

/// A sink for an origin-preserving command stream.
///
/// The sink forwards effects and structured command events into a
/// [`CommandContext`]. It expects events to already have the correct owner
/// context for the destination command type.
pub(crate) struct CommandSink<Effect, Event> {
    pub(crate) context: CommandContext<Effect, Event>,
}

impl<Effect, Event> CommandSink<Effect, Event> {
    pub(crate) const fn new(context: CommandContext<Effect, Event>) -> Self {
        Self { context }
    }
}

#[derive(Debug, Error)]
pub(crate) enum HostedCommandError {
    /// The host command's effect queue has been disconnected.
    #[error("Cannot send effect to host")]
    CannotSendEffect,
    /// The host command's event queue has been disconnected.
    #[error("Cannot send event to host")]
    CannotSendEvent,
}

impl<Effect, Event> Sink<CommandOutput<Effect, CommandEvent<Effect, Event>>>
    for CommandSink<Effect, Event>
{
    type Error = HostedCommandError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(
        self: Pin<&mut Self>,
        item: CommandOutput<Effect, CommandEvent<Effect, Event>>,
    ) -> Result<(), Self::Error> {
        match item {
            CommandOutput::Effect(effect) => self
                .context
                .effects
                .send(effect)
                .map_err(|_| HostedCommandError::CannotSendEffect),
            CommandOutput::Event(event) => self
                .context
                .events
                .send(event)
                .map_err(|_| HostedCommandError::CannotSendEvent),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

pub(crate) trait CommandStreamExt<Effect, Event>:
    Stream<Item = CommandOutput<Effect, CommandEvent<Effect, Event>>>
{
    /// Forward this origin-preserving command stream into a command context.
    ///
    /// This is used to multiplex hosted or mapped commands into the effect and
    /// event queues of their host command.
    fn host(self, context: CommandContext<Effect, Event>) -> impl Future
    where
        Self: Send + Sized,
    {
        self.map(Ok).forward(CommandSink::new(context))
    }
}

impl<S, Effect, Event> CommandStreamExt<Effect, Event> for S where
    S: Stream<Item = CommandOutput<Effect, CommandEvent<Effect, Event>>>
{
}
