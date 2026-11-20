#![allow(clippy::redundant_pub_crate)]
//! Stream support for [`Command`].
//!
//! [`Command::into_stream`] yields effect requests and events as they
//! become available. This is useful for testing, manual orchestration, and
//! advanced command composition.

use std::future::Future;
use std::task::{Context, Poll};

use std::pin::Pin;

use crate::core::CommandObservation;

use futures::{Sink, Stream, StreamExt as _};

use thiserror::Error;

use super::{Command, CommandContext, CommandEffect, CommandEvent};

impl<Effect, Event> Command<Effect, Event>
where
    Effect: Unpin + Send + 'static,
    Event: Unpin + Send + 'static,
{
    /// Consume this command as a stream of effects and events.
    pub fn into_stream(self) -> impl Stream<Item = CommandOutput<Effect, Event>> {
        self.into_stream_with_context().map(|output| match output {
            CommandOutput::Effect(effect) => CommandOutput::Effect(effect.into_effect()),
            CommandOutput::Event(event) => CommandOutput::Event(event.into_event()),
        })
    }

    /// Consume this command as a stream preserving observation context.
    pub(crate) fn into_stream_with_context(
        mut self,
    ) -> impl Stream<Item = CommandOutput<CommandEffect<Effect>, CommandEvent<Event>>> {
        futures::stream::poll_fn(move |cx| {
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
        })
    }

    /// Host a command and attach observation context to
    /// its outputs. Core dispatches effects after draining the hosted commands.
    ///
    /// Queued events carry this command's causal ID, without keeping it alive.
    /// The span ends when this command is cleaned up, independently of commands
    /// returned by later updates.
    pub(crate) fn host_with_observation(
        self,
        host_context: CommandContext<Effect, Event>,
        command_observation: CommandObservation,
    ) -> impl Future<Output = ()> {
        let id = command_observation.id;
        // Capture the tuple as a whole. Tuple fields drop in order: command
        // cleanup must precede span end, even if the host is never polled.
        let mut owned = (self.into_stream_with_context(), command_observation);
        let stream = futures::stream::poll_fn(move |cx| {
            let (stream, _) = &mut owned;
            stream.poll_next_unpin(cx)
        })
        .map(move |mut output| {
            match &mut output {
                CommandOutput::Event(event) => {
                    event.caused_by.get_or_insert(id);
                }
                CommandOutput::Effect(effect) => {
                    effect.parent_context_id.get_or_insert(id);
                }
            }
            output
        });
        async move {
            stream
                .host(host_context)
                .await
                .expect("event host must be able to forward its outputs");
        }
    }

    /// Host this command inside another command context, forwarding its effects
    /// and events. The host owns this command until it finishes or is dropped.
    pub(crate) fn host(self, context: CommandContext<Effect, Event>) -> impl Future {
        self.into_stream_with_context().host(context)
    }
}

/// An item emitted from a [`Command`] stream.
///
/// The public [`Command::into_stream`] method uses `Event` as the
/// event payload. Crux runtime internals use the same enum with
/// [`CommandEvent`] as the event payload, preserving its causal command ID.
#[derive(Debug)]
pub enum CommandOutput<Effect, Event> {
    /// An effect request for the shell or an effect handler to process.
    Effect(Effect),
    /// An event emitted by a command.
    Event(Event),
}

/// A sink for a context-preserving command stream.
///
/// The sink forwards effects and events with their observation context into
/// a [`CommandContext`].
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

impl<Effect, Event> Sink<CommandOutput<CommandEffect<Effect>, CommandEvent<Event>>>
    for CommandSink<Effect, Event>
{
    type Error = HostedCommandError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(
        self: Pin<&mut Self>,
        item: CommandOutput<CommandEffect<Effect>, CommandEvent<Event>>,
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
    Stream<Item = CommandOutput<CommandEffect<Effect>, CommandEvent<Event>>>
{
    /// Forward this context-preserving command stream into a command context.
    ///
    /// This is used to multiplex hosted or mapped commands into the effect and
    /// event queues of their host command.
    fn host(
        self,
        context: CommandContext<Effect, Event>,
    ) -> impl Future<Output = Result<(), HostedCommandError>>
    where
        Self: Send + Sized,
    {
        self.map(Ok).forward(CommandSink::new(context))
    }
}

impl<S, Effect, Event> CommandStreamExt<Effect, Event> for S where
    S: Stream<Item = CommandOutput<CommandEffect<Effect>, CommandEvent<Event>>>
{
}
