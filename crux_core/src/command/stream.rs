#![allow(clippy::redundant_pub_crate)]
//! Stream support for [`Command`].
//!
//! Commands implement [`Stream`], yielding effect requests and events as they
//! become available. This is useful for testing, manual orchestration, and
//! advanced command composition.

use core::future::Future;
use core::task::{Context, Poll};

use core::pin::Pin;

use futures::{Sink, Stream, StreamExt as _};

use thiserror::Error;

use super::{Command, CommandContext};

impl<Effect, Event> Command<Effect, Event>
where
    Effect: Unpin + Send + 'static,
    Event: Unpin + Send + 'static,
{
    /// Host this command inside another command context, forwarding its effects
    /// and events. The host owns this command until it finishes or is dropped.
    pub(crate) fn host(self, context: CommandContext<Effect, Event>) -> impl Future {
        CommandStreamExt::host(self, context)
    }
}

/// An item emitted from a [`Command`] stream.
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
        self.waker.register(cx.waker());

        // run_until_settled is idempotent
        self.run_until_settled();

        // Check events first to preserve the order in which items were emitted. This is because
        // sending events doesn't yield, and the next request/stream await point will be
        // reached in the same poll, so any follow up effects will _also_ be available
        if let Ok(event) = self.events.try_recv() {
            return Poll::Ready(Some(CommandOutput::Event(event)));
        }

        if let Ok(effect) = self.effects.try_recv() {
            return Poll::Ready(Some(CommandOutput::Effect(effect)));
        }

        if self.is_done() {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

/// Forwards a command's effects and events into its host's queues.
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

impl<Effect, Event> Sink<CommandOutput<Effect, Event>> for CommandSink<Effect, Event> {
    type Error = HostedCommandError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(
        self: Pin<&mut Self>,
        item: CommandOutput<Effect, Event>,
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
    Stream<Item = CommandOutput<Effect, Event>>
{
    /// Forward this command stream into a command context.
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
    S: Stream<Item = CommandOutput<Effect, Event>>
{
}
