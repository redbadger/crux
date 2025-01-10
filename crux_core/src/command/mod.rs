//! Command represents one or more side-effects, resulting in interactions with the shell.
//! Core creates Commands and returns them from the `update` function in response to events.
//! Commands can be created directly, but more often they will be created and returned
//! by capability APIs.
//!
//! A Command can execute side-effects in parallel, in sequence or a combination of both. To
//! allow this orchestration they provide both a simple synchronous API and access to an
//! asynchronous API.
//!
//! Command surfaces the effect requests and events sent in response with
//! the [`Command::effects`] and [`Command::events`] methods. These can be used when testing
//! the side effects requested by an `update` call.
//!
//! Internally, Command resembles [`FuturesUnordered`](futures::stream::FuturesUnordered):
//! it manages and polls a number of futures and provides a context which they can use
//! to submit effects to the shell and events back to the application.
//!
//! Command implements [`Stream`](futures::Stream), making it useful in an async context,
//! enabling, for example, wrapping Commands in one another.
//!
//! # Examples
//!
//! TODO: simple command example with a capability API
//!
//! TODO: complex example with sync API
//!
//! TODO: basic async example
//!
//! TODO: async example with `spawn`
//!
//! TODO: cancellation example
//!
//! TODO: testing example
//!
//! TODO: composition example

mod builder;
mod context;
mod executor;
mod stream;

use std::future::Future;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

// TODO: consider switching to flume
use crossbeam_channel::{Receiver, Sender};
use executor::{AbortHandle, Task, TaskId};
use futures::task::AtomicWaker;
use futures::{FutureExt as _, Stream, StreamExt as _};
use slab::Slab;
use stream::{CommandOutput, CommandStreamExt as _};

use crate::capability::Operation;
use crate::Request;

pub use builder::CommandBuilder;

pub struct Command<Effect, Event> {
    effects: Receiver<Effect>,
    events: Receiver<Event>,

    // Executor internals
    // TODO: should this be a separate type?
    ready_queue: Receiver<TaskId>,
    spawn_queue: Receiver<Task>,
    tasks: Slab<Task>,
    ready_sender: Sender<TaskId>, // Used in creating wakers for tasks
    waker: Arc<AtomicWaker>,      // Shared with task wakers when polled in async context

    // Signaling
    aborted: Arc<AtomicBool>,
}

// Public API

impl<Effect, Event> Command<Effect, Event>
where
    Effect: Send + 'static,
    Event: Send + 'static,
{
    /// Create a new command orchestrating effects with async Rust. This is the lowest level
    /// API to create a Command if you need full control over its execution. In most cases you will
    /// more likely want to create Commands with capabilities, and using the combinator APIs
    /// ([`then`], [`and`] and [`all`]) to orchestrate them.
    ///
    /// The `create_task` closure receives a [`CommandContext`] it can use to send shell request,
    /// events back to the app and spawn additional tasks. The closure is expected to return a future
    /// which becomes the command's main asynchronous task.
    pub fn new<F, Fut>(create_task: F) -> Self
    where
        F: FnOnce(context::CommandContext<Effect, Event>) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // RFC: do we need to think about backpressure? The channels are unbounded
        // so a naughty Command can make massive amounts of requests or spawn a huge number of tasks.
        // If these channels supported async, the CommandContext methods could also be async and
        // we could give the channels some bounds
        let (effect_sender, effect_receiver) = crossbeam_channel::unbounded();
        let (event_sender, event_receiver) = crossbeam_channel::unbounded();
        let (ready_sender, ready_receiver) = crossbeam_channel::unbounded();
        let (spawn_sender, spawn_receiver) = crossbeam_channel::unbounded();

        let context = context::CommandContext {
            effects: effect_sender,
            events: event_sender,
            tasks: spawn_sender,
        };

        let aborted: Arc<AtomicBool> = Default::default();
        let task = Task {
            join_handle_waker: Default::default(),
            finished: Default::default(),
            aborted: aborted.clone(),
            future: create_task(context).boxed(),
        };

        let mut tasks = Slab::with_capacity(1);
        let task_id = TaskId(tasks.insert(task));

        ready_sender
            .send(task_id)
            .expect("Could not make task ready, ready channel disconnected");

        Command {
            effects: effect_receiver,
            events: event_receiver,
            ready_queue: ready_receiver,
            spawn_queue: spawn_receiver,
            ready_sender,
            tasks,
            waker: Default::default(),
            aborted,
        }
    }

    /// Create an empty, completed Command. This is useful as a return value from `update` if
    /// there are no side-effects to perform.
    pub fn done() -> Self {
        let (_, effects) = crossbeam_channel::bounded(0);
        let (_, events) = crossbeam_channel::bounded(0);
        let (_, spawn_queue) = crossbeam_channel::bounded(0);
        let (ready_sender, ready_queue) = crossbeam_channel::bounded(0);

        Command {
            effects,
            events,
            ready_queue,
            spawn_queue,
            tasks: Slab::with_capacity(0),
            ready_sender,
            waker: Default::default(),
            aborted: Default::default(),
        }
    }

    /// Create a Command which dispatches an event and terminates. This is an alternative
    /// to calling `update` recursively. The only difference is that the two `update` calls
    /// will be visible to Crux and can show up in logs or any tooling. The trade-off is that
    /// the event is not guaranteed to dispatch instantly - another `update` call which is
    /// already scheduled may happen first.
    pub fn event(event: Event) -> Self {
        Command::new(|ctx| async move { ctx.send_event(event) })
    }

    /// Create a Command which sends a notification to the shell with a provided `operation`.
    ///
    /// This ia synchronous equivalent of [`CommandContext::notify_shell`].
    pub fn notify_shell<Op>(operation: Op) -> Command<Effect, Event>
    where
        Op: Operation,
        Effect: From<Request<Op>>,
    {
        Command::new(|ctx| async move { ctx.notify_shell(operation) })
    }

    /// Start a creation of a Command which sends a one-time request to the shell with a provided
    /// operation.
    ///
    /// Returns a `RequestBuilder`, which can be converted into a Command directly, or chained
    /// with another command builder using `.then`.
    ///
    /// In an async context, `RequestBuilder` can be turned into a future that resolves to the
    /// operation output type.
    pub fn request_from_shell<Op>(
        operation: Op,
    ) -> builder::RequestBuilder<Effect, Event, impl Future<Output = Op::Output>>
    where
        Op: Operation,
        Effect: From<Request<Op>>,
    {
        builder::RequestBuilder::new(|ctx| ctx.request_from_shell(operation))
    }

    /// Start a creation of a Command which sends a stream request to the shell with a provided
    /// operation.
    ///
    /// Returns a `StreamBuilder`, which can be converted into a Command directly, or chained
    /// with a `RequestBuilder` builder using `.then`.
    ///
    /// In an async context, `StreamBuilder` can be turned into a stream that with the
    /// operation output type as item.
    pub fn stream_from_shell<Op>(
        operation: Op,
    ) -> builder::StreamBuilder<Effect, Event, impl Stream<Item = Op::Output>>
    where
        Op: Operation,
        Effect: From<Request<Op>>,
    {
        builder::StreamBuilder::new(|ctx| ctx.stream_from_shell(operation))
    }

    /// Run the effect state machine until it settles, then return true
    /// if there is any more work to do - tasks to run or events or effects to receive
    pub fn is_done(&mut self) -> bool {
        self.run_until_settled();

        self.effects.is_empty() && self.events.is_empty() && self.tasks.is_empty()
    }

    /// Run the effect state machine until it settles and collect all effects generated
    pub fn effects(&mut self) -> impl Iterator<Item = Effect> + '_ {
        self.run_until_settled();

        self.effects.try_iter()
    }

    /// Run the effect state machine until it settles and collect all events generated
    pub fn events(&mut self) -> impl Iterator<Item = Event> + '_ {
        self.run_until_settled();

        self.events.try_iter()
    }

    // Combinators

    /// Create a command running self and the other command in sequence
    // RFC: is this actually _useful_? Unlike `.then` on `CommandBuilder` this doesn't allow using
    // the output of the first command in building the second one, it just runs them in sequence,
    // and the benefit is unclear.
    pub fn then(self, other: Self) -> Self
    where
        Effect: Unpin,
        Event: Unpin,
    {
        Command::new(|ctx| async move {
            // first run self until done
            self.host(ctx.effects.clone(), ctx.events.clone()).await;

            // then run other until done
            other.host(ctx.effects, ctx.events).await;
        })
    }

    /// Convenience for [`Command::all`] which runs another command concurrently with this one
    pub fn and(self, other: Self) -> Self
    where
        Effect: Unpin,
        Event: Unpin,
    {
        Command::all([self, other])
    }

    /// Create a command running a number of commands concurrently
    pub fn all<I>(commands: I) -> Self
    where
        I: IntoIterator<Item = Self> + Send + 'static,
        Effect: Unpin,
        Event: Unpin,
    {
        Command::new(|ctx| async move {
            let select = futures::stream::select_all(commands);

            select.host(ctx.effects, ctx.events).await;
        })
    }

    // Mapping for composition

    /// Map effects requested as part of this command to a different effect type.
    ///
    /// This is useful when composing apps to convert a command from a child app to a
    /// command of the parent app.
    pub fn map_effect<F, NewEffect>(self, map: F) -> Command<NewEffect, Event>
    where
        F: Fn(Effect) -> NewEffect + Send + Sync + 'static,
        NewEffect: Send + Unpin + 'static,
        Effect: Unpin,
        Event: Unpin,
    {
        Command::new(|ctx| async move {
            let mapped = self.map(|output| match output {
                CommandOutput::Effect(effect) => CommandOutput::Effect(map(effect)),
                CommandOutput::Event(event) => CommandOutput::Event(event),
            });

            mapped.host(ctx.effects, ctx.events).await;
        })
    }

    /// Map events sent as part of this command to a different effect type
    ///
    /// This is useful when composing apps to convert a command from a child app to a
    /// command of the parent app.
    pub fn map_event<F, NewEvent>(self, map: F) -> Command<Effect, NewEvent>
    where
        F: Fn(Event) -> NewEvent + Send + Sync + 'static,
        NewEvent: Send + Unpin + 'static,
        Effect: Unpin,
        Event: Unpin,
    {
        Command::new(|ctx| async move {
            let mapped = self.map(|output| match output {
                CommandOutput::Effect(effect) => CommandOutput::Effect(effect),
                CommandOutput::Event(event) => CommandOutput::Event(map(event)),
            });

            mapped.host(ctx.effects, ctx.events).await;
        })
    }

    /// Returns an abort handle which can be used to remotely terminate a running Command
    /// and all its subtask.
    ///
    /// This is specifically useful for cancelling subscriptions and long running effects
    /// which may get superseded, like timers
    pub fn abort_handle(&self) -> AbortHandle {
        AbortHandle {
            aborted: self.aborted.clone(),
        }
    }
}

#[cfg(test)]
mod tests;
