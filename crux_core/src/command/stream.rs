// Command is an async Stream

use std::future::Future;
use std::ops::DerefMut as _;
use std::task::{Context, Poll};

use std::pin::Pin;

use futures::{Sink, Stream, StreamExt as _};

use crossbeam_channel::Sender;
use thiserror::Error;

use super::Command;

/// An item emitted from a Command when used as a Stream.
#[derive(Debug)]
pub enum CommandOutput<Effect, Event> {
    Effect(Effect),
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
        self.deref_mut().run_until_settled();

        // Check events first to preserve the order in which items were emitted. This is because
        // sending events doesn't yield, and the next request/stream await point will be
        // reached in the same poll, so any follow up effects will _also_ be available
        if let Ok(event) = self.events.try_recv() {
            return Poll::Ready(Some(CommandOutput::Event(event)));
        }

        if let Ok(effect) = self.effects.try_recv() {
            return Poll::Ready(Some(CommandOutput::Effect(effect)));
        };

        if self.is_done() {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

/// A sink for a Command stream, sending all emitted effects and events into a pair of channels
pub(crate) struct CommandSink<Effect, Event> {
    pub(crate) effects: Sender<Effect>,
    pub(crate) events: Sender<Event>,
}

impl<Effect, Event> CommandSink<Effect, Event> {
    pub(crate) fn new(effects: Sender<Effect>, events: Sender<Event>) -> Self {
        Self { effects, events }
    }
}

#[derive(Debug, Error)]
pub(crate) enum HostedCommandError {
    #[error("Cannot send effect to host")]
    CannotSendEffect,
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
                .effects
                .send(effect)
                .map_err(|_| HostedCommandError::CannotSendEffect),
            CommandOutput::Event(event) => self
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
    /// Connect this command to a pair of effect and event channels
    ///
    /// This is useful if you need to multiplex several commands into the same stream of
    /// effects and events - like Crux does.
    fn host(self, effects: Sender<Effect>, events: Sender<Event>) -> impl Future
    where
        Self: Send + Sized,
    {
        self.map(Ok).forward(CommandSink::new(effects, events))
    }
}

impl<S, Effect, Event> CommandStreamExt<Effect, Event> for S where
    S: Stream<Item = CommandOutput<Effect, Event>>
{
}
