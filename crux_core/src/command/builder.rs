//! Command builders are an abstraction allowing chaining effects,
//! where outputs of one effect can serve as inputs to further effects,
//! without requiring an async context.
//!
//! Chaining streams with streams is currently not supported, as the semantics
//! of the composition are unclear. If you need to compose streams, use the async
//! API and tools from the `futures` crate.

use std::{future::Future, pin::pin};

use futures::{FutureExt, Stream, StreamExt};

use super::{context::CommandContext, Command};

/// A builder of one-off notify command
// Task is a future which does the shell talking and returns an output
pub struct NotificationBuilder<Effect, Event, Task> {
    make_task: Box<dyn FnOnce(CommandContext<Effect, Event>) -> Task + Send>,
}

impl<Effect, Event, Task> NotificationBuilder<Effect, Event, Task>
where
    Effect: Send + 'static,
    Event: Send + 'static,
    Task: Future<Output = ()> + Send + 'static,
{
    pub fn new<F>(make_task: F) -> Self
    where
        F: FnOnce(CommandContext<Effect, Event>) -> Task + Send + 'static,
    {
        let make_task = Box::new(make_task);

        NotificationBuilder { make_task }
    }

    /// Convert the [`NotificationBuilder`] into a future to use in an async context
    pub fn into_future(self, ctx: CommandContext<Effect, Event>) -> Task {
        let make_task = self.make_task;
        make_task(ctx)
    }
}

impl<Effect, Event, Task> From<NotificationBuilder<Effect, Event, Task>> for Command<Effect, Event>
where
    Effect: Send + 'static,
    Event: Send + 'static,
    Task: Future<Output = ()> + Send + 'static,
{
    fn from(value: NotificationBuilder<Effect, Event, Task>) -> Self {
        Command::new(|ctx| value.into_future(ctx))
    }
}

/// A builder of one-off request command
// Task is a future which does the shell talking and returns an output
pub struct RequestBuilder<Effect, Event, Task> {
    make_task: Box<dyn FnOnce(CommandContext<Effect, Event>) -> Task + Send>,
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

    pub fn map<F, U>(self, map: F) -> RequestBuilder<Effect, Event, impl Future<Output = U>>
    where
        F: FnOnce(T) -> U + Send + 'static,
    {
        RequestBuilder::new(|ctx| self.into_future(ctx.clone()).map(map))
    }

    /// Chain another [`RequestBuilder`] to run after completion of this one,
    /// passing the result to the provided closure `make_next_builder`.
    ///
    /// The returned value of the closure must be a `RequestBuilder`, which
    /// can represent some more work to be done before the composed future
    /// is finished.
    ///
    /// If you want to chain a subscription, use [`Self::then_stream`] instead.
    ///
    /// The closure `make_next_builder` is only run *after* successful completion
    /// of the `self` future.
    ///
    /// Note that this function consumes the receiving `RequestBuilder` and returns a
    /// new one that represents the composition.
    ///
    /// # Example
    ///
    /// ```
    /// # use crux_core::{Command, Request};
    /// # use crux_core::capability::Operation;
    /// # use serde::{Deserialize, Serialize};
    /// # #[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
    /// # enum AnOperation {
    /// #     One,
    /// #     Two,
    /// #     More(u8),
    /// # }
    /// #
    /// # #[derive(Debug, PartialEq, Deserialize)]
    /// # enum AnOperationOutput {
    /// #     One,
    /// #     Two,
    /// #     Other(u8),
    /// # }
    /// #
    /// # impl Operation for AnOperation {
    /// #     type Output = AnOperationOutput;
    /// # }
    /// #
    /// # #[derive(Debug)]
    /// # enum Effect {
    /// #     AnEffect(Request<AnOperation>),
    /// # }
    /// #
    /// # impl From<Request<AnOperation>> for Effect {
    /// #     fn from(request: Request<AnOperation>) -> Self {
    /// #         Self::AnEffect(request)
    /// #     }
    /// # }
    /// #
    /// # #[derive(Debug, PartialEq)]
    /// # enum Event {
    /// #     Completed(AnOperationOutput),
    /// # }
    /// let mut cmd: Command<Effect, Event> = Command::request_from_shell(AnOperation::More(1))
    ///     .then_request(|first| {
    ///         let AnOperationOutput::Other(first) = first else {
    ///             panic!("Invalid output!")
    ///         };
    ///
    ///         let second = first + 1;
    ///         Command::request_from_shell(AnOperation::More(second))
    ///     })
    ///     .then_send(Event::Completed);
    ///
    /// let Effect::AnEffect(mut request) = cmd.effects().next().unwrap();
    /// assert_eq!(request.operation, AnOperation::More(1));
    ///
    /// request
    ///    .resolve(AnOperationOutput::Other(1))
    ///    .expect("to resolve");
    ///
    /// let Effect::AnEffect(mut request) = cmd.effects().next().unwrap();
    /// assert_eq!(request.operation, AnOperation::More(2));
    /// ```
    pub fn then_request<F, U, NextTask>(
        self,
        make_next_builder: F,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = U>>
    where
        F: FnOnce(T) -> RequestBuilder<Effect, Event, NextTask> + Send + 'static,
        NextTask: Future<Output = U> + Send + 'static,
    {
        RequestBuilder::new(|ctx| {
            self.into_future(ctx.clone())
                .then(|out| make_next_builder(out).into_future(ctx))
        })
    }

    /// Chain a [`StreamBuilder`] to run after completion of this [`RequestBuilder`],
    /// passing the result to the provided closure `make_next_builder`.
    ///
    /// The returned value of the closure must be a `StreamBuilder`, which
    /// can represent some more work to be done before the composed future
    /// is finished.
    ///
    /// If you want to chain a request, use [`Self::then_request`] instead.
    ///
    /// The closure `make_next_builder` is only run *after* successful completion
    /// of the `self` future.
    ///
    /// Note that this function consumes the receiving `RequestBuilder` and returns a
    /// [`StreamBuilder`] that represents the composition.
    ///
    /// # Example
    ///
    /// ```
    /// # use crux_core::{Command, Request};
    /// # use crux_core::capability::Operation;
    /// # use serde::{Deserialize, Serialize};
    /// # #[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
    /// # enum AnOperation {
    /// #     One,
    /// #     Two,
    /// #     More(u8),
    /// # }
    /// #
    /// # #[derive(Debug, PartialEq, Deserialize)]
    /// # enum AnOperationOutput {
    /// #     One,
    /// #     Two,
    /// #     Other(u8),
    /// # }
    /// #
    /// # impl Operation for AnOperation {
    /// #     type Output = AnOperationOutput;
    /// # }
    /// #
    /// # #[derive(Debug)]
    /// # enum Effect {
    /// #     AnEffect(Request<AnOperation>),
    /// # }
    /// #
    /// # impl From<Request<AnOperation>> for Effect {
    /// #     fn from(request: Request<AnOperation>) -> Self {
    /// #         Self::AnEffect(request)
    /// #     }
    /// # }
    /// #
    /// # #[derive(Debug, PartialEq)]
    /// # enum Event {
    /// #     Completed(AnOperationOutput),
    /// # }
    /// let mut cmd: Command<Effect, Event> = Command::request_from_shell(AnOperation::More(1))
    ///    .then_stream(|first| {
    ///       let AnOperationOutput::Other(first) = first else {
    ///          panic!("Invalid output!")
    ///      };
    ///
    ///      let second = first + 1;
    ///      Command::stream_from_shell(AnOperation::More(second))
    ///    })
    ///    .then_send(Event::Completed);
    ///
    /// let Effect::AnEffect(mut request) = cmd.effects().next().unwrap();
    /// assert_eq!(request.operation, AnOperation::More(1));
    ///
    /// request
    ///   .resolve(AnOperationOutput::Other(1))
    ///   .expect("to resolve");
    ///
    /// let Effect::AnEffect(mut request) = cmd.effects().next().unwrap();
    /// assert_eq!(request.operation, AnOperation::More(2));
    pub fn then_stream<F, U, NextTask>(
        self,
        make_next_builder: F,
    ) -> StreamBuilder<Effect, Event, impl Stream<Item = U>>
    where
        F: FnOnce(T) -> StreamBuilder<Effect, Event, NextTask> + Send + 'static,
        NextTask: Stream<Item = U> + Send + 'static,
    {
        StreamBuilder::new(|ctx| {
            self.into_future(ctx.clone())
                .map(make_next_builder)
                .into_stream()
                .flat_map(move |builder| builder.into_stream(ctx.clone()))
        })
    }

    /// Convert the request builder into a future to use in an async context
    pub fn into_future(self, ctx: CommandContext<Effect, Event>) -> Task {
        let make_task = self.make_task;
        make_task(ctx)
    }

    /// Create the command in an evented context
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

    pub fn map<F, U>(self, map: F) -> StreamBuilder<Effect, Event, impl Stream<Item = U>>
    where
        F: FnMut(T) -> U + Send + 'static,
    {
        StreamBuilder::new(|ctx| self.into_stream(ctx.clone()).map(map))
    }

    /// Chain a [`RequestBuilder`] to run after completion of this [`StreamBuilder`],
    /// passing the result to the provided closure `make_next_builder`.
    ///
    /// The returned value of the closure must be a [`StreamBuilder`], which
    /// can represent some more work to be done before the composed future
    /// is finished.
    ///
    /// If you want to chain a subscription, use [`Self::then_stream`] instead.
    ///
    /// The closure `make_next_builder` is only run *after* successful completion
    /// of the `self` future.
    ///
    /// Note that this function consumes the receiving `StreamBuilder` and returns a
    /// new one that represents the composition.
    ///
    /// # Example
    ///
    /// ```
    /// # use crux_core::{Command, Request};
    /// # use crux_core::capability::Operation;
    /// # use serde::{Deserialize, Serialize};
    /// # #[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
    /// # enum AnOperation {
    /// #     One,
    /// #     Two,
    /// #     More(u8),
    /// # }
    /// #
    /// # #[derive(Debug, PartialEq, Deserialize)]
    /// # enum AnOperationOutput {
    /// #     One,
    /// #     Two,
    /// #     Other(u8),
    /// # }
    /// #
    /// # impl Operation for AnOperation {
    /// #     type Output = AnOperationOutput;
    /// # }
    /// #
    /// # #[derive(Debug)]
    /// # enum Effect {
    /// #     AnEffect(Request<AnOperation>),
    /// # }
    /// #
    /// # impl From<Request<AnOperation>> for Effect {
    /// #     fn from(request: Request<AnOperation>) -> Self {
    /// #         Self::AnEffect(request)
    /// #     }
    /// # }
    /// #
    /// # #[derive(Debug, PartialEq)]
    /// # enum Event {
    /// #     Completed(AnOperationOutput),
    /// # }
    /// let mut cmd: Command<Effect, Event> = Command::stream_from_shell(AnOperation::More(1))
    ///     .then_request(|first| {
    ///         let AnOperationOutput::Other(first) = first else {
    ///             panic!("Invalid output!")
    ///         };
    ///
    ///         let second = first + 1;
    ///         Command::request_from_shell(AnOperation::More(second))
    ///     })
    ///     .then_send(Event::Completed);
    ///
    /// let Effect::AnEffect(mut request) = cmd.effects().next().unwrap();
    /// assert_eq!(request.operation, AnOperation::More(1));
    ///
    /// request
    ///    .resolve(AnOperationOutput::Other(1))
    ///    .expect("to resolve");
    ///
    /// let Effect::AnEffect(mut request) = cmd.effects().next().unwrap();
    /// assert_eq!(request.operation, AnOperation::More(2));
    /// ```
    pub fn then_request<F, U, NextTask>(
        self,
        make_next_builder: F,
    ) -> StreamBuilder<Effect, Event, impl Stream<Item = U>>
    where
        F: Fn(T) -> RequestBuilder<Effect, Event, NextTask> + Send + 'static,
        NextTask: Future<Output = U> + Send + 'static,
    {
        StreamBuilder::new(|ctx| {
            self.into_stream(ctx.clone())
                .then(move |item| make_next_builder(item).into_future(ctx.clone()))
        })
    }

    /// Chain another [`StreamBuilder`] to run after completion of this one,
    /// passing the result to the provided closure `make_next_builder`.
    ///
    /// The returned value of the closure must be a `StreamBuilder`, which
    /// can represent some more work to be done before the composed future
    /// is finished.
    ///
    /// If you want to chain a request, use [`Self::then_request`] instead.
    ///
    /// The closure `make_next_builder` is only run *after* successful completion
    /// of the `self` future.
    ///
    /// Note that this function consumes the receiving `StreamBuilder` and returns a
    /// new one that represents the composition.
    ///
    /// # Example
    ///
    /// ```
    /// # use crux_core::{Command, Request};
    /// # use crux_core::capability::Operation;
    /// # use serde::{Deserialize, Serialize};
    /// # #[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
    /// # enum AnOperation {
    /// #     One,
    /// #     Two,
    /// #     More(u8),
    /// # }
    /// #
    /// # #[derive(Debug, PartialEq, Deserialize)]
    /// # enum AnOperationOutput {
    /// #     One,
    /// #     Two,
    /// #     Other(u8),
    /// # }
    /// #
    /// # impl Operation for AnOperation {
    /// #     type Output = AnOperationOutput;
    /// # }
    /// #
    /// # #[derive(Debug)]
    /// # enum Effect {
    /// #     AnEffect(Request<AnOperation>),
    /// # }
    /// #
    /// # impl From<Request<AnOperation>> for Effect {
    /// #     fn from(request: Request<AnOperation>) -> Self {
    /// #         Self::AnEffect(request)
    /// #     }
    /// # }
    /// #
    /// # #[derive(Debug, PartialEq)]
    /// # enum Event {
    /// #     Completed(AnOperationOutput),
    /// # }
    /// let mut cmd: Command<Effect, Event> = Command::stream_from_shell(AnOperation::More(1))
    ///    .then_stream(|first| {
    ///       let AnOperationOutput::Other(first) = first else {
    ///          panic!("Invalid output!")
    ///      };
    ///
    ///      let second = first + 1;
    ///      Command::stream_from_shell(AnOperation::More(second))
    ///    })
    ///    .then_send(Event::Completed);
    ///
    /// let Effect::AnEffect(mut request) = cmd.effects().next().unwrap();
    /// assert_eq!(request.operation, AnOperation::More(1));
    ///
    /// request
    ///   .resolve(AnOperationOutput::Other(1))
    ///   .expect("to resolve");
    ///
    /// let Effect::AnEffect(mut request) = cmd.effects().next().unwrap();
    /// assert_eq!(request.operation, AnOperation::More(2));
    pub fn then_stream<F, U, NextTask>(
        self,
        make_next_builder: F,
    ) -> StreamBuilder<Effect, Event, impl Stream<Item = U>>
    where
        F: Fn(T) -> StreamBuilder<Effect, Event, NextTask> + Send + 'static,
        NextTask: Stream<Item = U> + Send + 'static,
    {
        StreamBuilder::new(move |ctx| {
            self.into_stream(ctx.clone())
                .map(move |item| {
                    let next_builder = make_next_builder(item);
                    Box::pin(next_builder.into_stream(ctx.clone()))
                })
                .flatten_unordered(None)
        })
    }

    /// Create the command in an evented context
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

    /// Convert the stream builder into a stream to use in an async context
    pub fn into_stream(self, ctx: CommandContext<Effect, Event>) -> Task {
        let make_stream = self.make_stream;

        make_stream(ctx)
    }
}
