//! Command builders are an abstraction allowing chaining effects,
//! where outputs of one effect can serve as inputs to further effects,
//! without requiring an async context.
//!
//! Only simple chaining is supported by this API:
//! * Request responses can be used for further requests
//! * Request responses can be used to start a stream
//! * Stream output can be used to issue requests
//!
//! Chaining streams with streams is currently not supported, as the semantics
//! of the composition are unclear. If you need to compose streams, use the async
//! API and tools from the `futures` crate.

use std::{future::Future, pin::pin};

use futures::{FutureExt, Stream, StreamExt};

use super::{context::CommandContext, Command};

/// A common behaviour for RequestBuilder and Stream builder
pub trait CommandBuilder<Effect, Event, T> {
    type Task;

    fn new_with_context<F, AsyncTask>(make_task: F) -> impl CommandBuilder<Effect, Event, T>
    where
        F: FnOnce(CommandContext<Effect, Event>) -> AsyncTask + Send + 'static,
        AsyncTask: Future<Output = Self::Task> + Send + 'static;

    /// Convert the builder into its async representation - a Future or a Stream
    fn into_async(self, ctx: CommandContext<Effect, Event>) -> Self::Task;

    // FIXME: add a `.then` method so that the chaining can be infinite

    /// Convert the builder into a command which sends the event returned by the provided
    /// closure upon resolution
    fn then_send<E>(self, make_event: E) -> Command<Effect, Event>
    where
        E: Fn(T) -> Event + Send + 'static;
}

/// A builder of one-off request command
// Task is a future which does the shell talking and returns an output
pub struct RequestBuilder<Effect, Event, Task> {
    make_task: Box<dyn FnOnce(CommandContext<Effect, Event>) -> Task + Send>,
}

impl<Effect, Event, Task, T> CommandBuilder<Effect, Event, T>
    for RequestBuilder<Effect, Event, Task>
where
    Task: Future<Output = T> + Send + 'static,
    Effect: Send + 'static,
    Event: Send + 'static,
{
    type Task = Task;

    fn new_with_context<F, AsyncTask>(make_task: F) -> impl CommandBuilder<Effect, Event, T>
    where
        F: FnOnce(CommandContext<Effect, Event>) -> AsyncTask + Send + 'static,
        AsyncTask: Future<Output = Task> + Send + 'static,
    {
        RequestBuilder::new(|ctx| make_task(ctx).flatten())
    }

    fn into_async(self, ctx: CommandContext<Effect, Event>) -> Self::Task {
        self.into_future(ctx)
    }

    fn then_send<E>(self, make_event: E) -> Command<Effect, Event>
    where
        E: Fn(T) -> Event + Send + 'static,
    {
        Command::new(|ctx| async move {
            let out = self.into_future(ctx.clone()).await;

            ctx.send_event(make_event(out));
        })
    }
}

impl<Effect, Event, Task, T> RequestBuilder<Effect, Event, Task>
where
    Effect: Send + 'static,
    Event: Send + 'static,
    Task: Future<Output = T> + Send + 'static,
{
    pub fn new<F>(make_task: F) -> Self
    where
        F: FnOnce(CommandContext<Effect, Event>) -> Task + Send + 'static,
    {
        let make_task = Box::new(make_task);

        RequestBuilder { make_task }
    }

    pub fn then<F, NextBuilder, U>(
        self,
        make_next_builder: F,
    ) -> impl CommandBuilder<Effect, Event, U>
    where
        F: FnOnce(T) -> NextBuilder + Send + 'static,
        NextBuilder: CommandBuilder<Effect, Event, U>,
    {
        NextBuilder::new_with_context(|ctx| async {
            let out = self.into_future(ctx.clone()).await;

            make_next_builder(out).into_async(ctx)
        })
    }

    // Create the command in an async context
    pub fn into_future(self, ctx: CommandContext<Effect, Event>) -> Task {
        let make_task = self.make_task;
        make_task(ctx)
    }

    // Create the command in an evented context
    pub fn then_send<E>(self, event: E) -> Command<Effect, Event>
    where
        E: FnOnce(T) -> Event + Send + 'static,
        Task: Future<Output = T> + Send + 'static,
    {
        Command::new(|ctx| async move {
            let out = self.into_future(ctx.clone()).await;

            ctx.send_event(event(out));
        })
    }
}

/// A builder of stream command
pub struct StreamBuilder<Effect, Event, Task> {
    make_stream: Box<dyn FnOnce(CommandContext<Effect, Event>) -> Task + Send>,
}

impl<Effect, Event, Task, T> CommandBuilder<Effect, Event, T> for StreamBuilder<Effect, Event, Task>
where
    Task: Stream<Item = T> + Send + 'static,
    Effect: Send + 'static,
    Event: Send + 'static,
{
    type Task = Task;

    fn new_with_context<F, AsyncTask>(make_task: F) -> impl CommandBuilder<Effect, Event, T>
    where
        F: FnOnce(CommandContext<Effect, Event>) -> AsyncTask + Send + 'static,
        AsyncTask: Future<Output = Task> + Send + 'static,
    {
        StreamBuilder::new(|ctx| make_task(ctx.clone()).flatten_stream())
    }

    fn into_async(self, ctx: CommandContext<Effect, Event>) -> Task {
        self.into_stream(ctx)
    }

    fn then_send<E>(self, make_event: E) -> Command<Effect, Event>
    where
        E: Fn(T) -> Event + Send + 'static,
    {
        Command::new(|ctx| async move {
            let mut stream = pin!(self.into_stream(ctx.clone()));

            while let Some(out) = stream.next().await {
                ctx.send_event(make_event(out));
            }
        })
    }
}

impl<Effect, Event, Task, T> StreamBuilder<Effect, Event, Task>
where
    Effect: Send + 'static,
    Event: Send + 'static,
    Task: Stream<Item = T> + Send + 'static,
{
    pub fn new<F>(make_task: F) -> Self
    where
        F: FnOnce(CommandContext<Effect, Event>) -> Task + Send + 'static,
    {
        let make_task = Box::new(make_task);

        StreamBuilder {
            make_stream: make_task,
        }
    }

    pub fn then<Out, F, U>(self, next: F) -> StreamBuilder<Effect, Event, impl Stream<Item = U>>
    where
        F: Fn(T) -> RequestBuilder<Effect, Event, Out> + Clone + Send + Sync + 'static,
        Out: Future<Output = U> + Send + 'static,
    {
        StreamBuilder::new(move |ctx| {
            self.into_stream(ctx.clone()).then({
                let next = next.clone();
                move |item| {
                    let request = next(item);
                    let make_task = request.make_task;

                    make_task(ctx.clone())
                }
            })
        })
    }

    pub fn then_send<E>(self, event: E) -> Command<Effect, Event>
    where
        E: Fn(T) -> Event + Send + 'static,
    {
        Command::new(|ctx| async move {
            let mut stream = pin!(self.into_stream(ctx.clone()));

            while let Some(out) = stream.next().await {
                ctx.send_event(event(out));
            }
        })
    }

    pub fn into_stream(self, ctx: CommandContext<Effect, Event>) -> Task {
        let make_stream = self.make_stream;

        make_stream(ctx)
    }
}
