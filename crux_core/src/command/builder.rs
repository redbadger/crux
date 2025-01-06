use std::{future::Future, pin::pin};

use futures::{FutureExt, Stream, StreamExt};

use super::{Command, CommandContext};

pub trait CommandBuilder<Effect, Event, T> {
    type Task;

    fn new_with_context<F, AsyncTask>(make_task: F) -> impl CommandBuilder<Effect, Event, T>
    where
        F: FnOnce(CommandContext<Effect, Event>) -> AsyncTask + Send + 'static,
        AsyncTask: Future<Output = Self::Task> + Send + 'static;

    fn into_async(self, ctx: CommandContext<Effect, Event>) -> Self::Task;

    fn then_send<E>(self, make_event: E) -> Command<Effect, Event>
    where
        E: Fn(T) -> Event + Send + 'static;
}

// Task is a future which does the shell talking and returns an output
pub struct ShellRequest<Effect, Event, Task> {
    make_task: Box<dyn FnOnce(CommandContext<Effect, Event>) -> Task + Send>,
}

impl<Effect, Event, Task, T> CommandBuilder<Effect, Event, T> for ShellRequest<Effect, Event, Task>
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
        ShellRequest::new(|ctx| make_task(ctx).flatten())
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

impl<Effect, Event, Task, T> ShellRequest<Effect, Event, Task>
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

        ShellRequest { make_task }
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

pub struct ShellStream<Effect, Event, Task> {
    make_stream: Box<dyn FnOnce(CommandContext<Effect, Event>) -> Task + Send>,
}

impl<Effect, Event, Task, T> CommandBuilder<Effect, Event, T> for ShellStream<Effect, Event, Task>
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
        ShellStream::new(|ctx| make_task(ctx.clone()).flatten_stream())
    }

    fn into_async(self, ctx: CommandContext<Effect, Event>) -> Self::Task {
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

impl<Effect, Event, Task> ShellStream<Effect, Event, Task>
where
    Effect: Send + 'static,
    Event: Send + 'static,
{
    pub fn new<F>(make_task: F) -> Self
    where
        F: FnOnce(CommandContext<Effect, Event>) -> Task + Send + 'static,
    {
        let make_task = Box::new(make_task);

        ShellStream {
            make_stream: make_task,
        }
    }

    pub fn then<T, U, Out, F>(self, next: F) -> ShellStream<Effect, Event, impl Stream<Item = U>>
    where
        Task: Stream<Item = T> + 'static,
        F: Fn(T) -> ShellRequest<Effect, Event, Out> + Clone + Send + Sync + 'static,
        Out: Future<Output = U>,
    {
        ShellStream::new(move |ctx| {
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

    pub fn then_send<T, E>(self, event: E) -> Command<Effect, Event>
    where
        E: Fn(T) -> Event + Send + 'static,
        Task: Stream<Item = T> + Unpin + Send + 'static,
    {
        Command::new(|ctx| async move {
            let mut stream = self.into_stream(ctx.clone());

            while let Some(out) = stream.next().await {
                ctx.send_event(event(out));
            }
        })
    }

    pub fn into_stream<T>(self, ctx: CommandContext<Effect, Event>) -> Task
    where
        Task: Stream<Item = T>,
    {
        let make_stream = self.make_stream;

        make_stream(ctx)
    }
}
