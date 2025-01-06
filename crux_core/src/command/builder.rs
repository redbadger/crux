use std::future::Future;

use futures::{FutureExt, Stream, StreamExt};

use super::{Command, CommandContext};

pub struct ShellRequest<Effect, Event, Task> {
    make_task: Box<dyn FnOnce(CommandContext<Effect, Event>) -> Task + Send>,
}

impl<Effect, Event, Task, T> ShellRequest<Effect, Event, Task>
where
    Effect: Send + 'static,
    Event: Send + 'static,
    Task: Future<Output = T> + 'static,
{
    pub fn new<F>(make_task: F) -> Self
    where
        F: FnOnce(CommandContext<Effect, Event>) -> Task + Send + 'static,
    {
        let make_task = Box::new(make_task);

        ShellRequest { make_task }
    }

    pub fn then_request<F, NextTask, U>(
        self,
        make_next_builder: F,
    ) -> ShellRequest<Effect, Event, impl Future<Output = U>>
    where
        F: FnOnce(T) -> ShellRequest<Effect, Event, NextTask> + Send + 'static, // User's closure taking T and returning the next builder (request or stream builder)
        NextTask: Future<Output = U> + 'static,
    {
        ShellRequest::new(|ctx| {
            self.into_future(ctx.clone()).then(move |out| {
                let next_builder = make_next_builder(out);

                next_builder.into_future(ctx.clone())
            })
        })
    }

    pub fn then_stream<F, NextTask, U>(
        self,
        make_next_builder: F,
    ) -> ShellStream<Effect, Event, impl Stream<Item = U>>
    where
        F: Fn(T) -> ShellStream<Effect, Event, NextTask> + Send + Sync + 'static, // User's closure taking T and returning the next builder (request or stream builder)
        NextTask: Stream<Item = U>,
    {
        ShellStream::new(move |ctx| {
            self.into_future(ctx.clone())
                .map(move |out| make_next_builder(out).into_stream(ctx))
                .flatten_stream()
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

    pub fn then_request<T, U, Out, F>(
        self,
        next: F,
    ) -> ShellStream<Effect, Event, impl Stream<Item = U>>
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
