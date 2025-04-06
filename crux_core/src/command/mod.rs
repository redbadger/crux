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
//! Command implements [`Stream`], making it useful in an async context,
//! enabling, for example, wrapping Commands in one another.
//!
//! # Examples
//!
//! Commands are typically created by a capability and returned from the update function. Capabilities
//! normally return a builder, which can be used in both sync and async context. The basic sync use
//! is to bind the command to an Event which will be sent with the result of the command:
//!
//! ```
//!# use url::Url;
//!# use crux_core::{Command, render};
//!# use crux_http::command::Http;
//!# const API_URL: &str = "https://example.com/";
//!# pub enum Event { Increment, Set(crux_http::Result<crux_http::Response<usize>>) }
//!# #[derive(crux_core::macros::Effect)]
//!# pub struct Capabilities {
//!#     pub render: crux_core::render::Render<Event>,
//!#     pub http: crux_http::Http<Event>,
//!# }
//!# #[derive(Default)] pub struct Model { count: usize }
//!# #[derive(Default)] pub struct App;
//!# impl crux_core::App for App {
//!#     type Event = Event;
//!#     type Model = Model;
//!#     type ViewModel = ();
//!#     type Capabilities = Capabilities;
//!#     type Effect = Effect;
//! fn update(
//!     &self,
//!     event: Self::Event,
//!     model: &mut Self::Model,
//!     _caps: &Self::Capabilities)
//! -> Command<Effect, Event> {
//!     match event {
//!         //...
//!         Event::Increment => {
//!             let base = Url::parse(API_URL).unwrap();
//!             let url = base.join("/inc").unwrap();
//!
//!             Http::post(url)            // creates an HTTP RequestBuilder
//!                 .expect_json()
//!                 .build()               // creates a Command RequestBuilder
//!                 .then_send(Event::Set) // creates a Command
//!         }
//!         Event::Set(Ok(mut response)) => {
//!              let count = response.take_body().unwrap();
//!              model.count = count;
//!              render::render()
//!         }
//!         Event::Set(Err(_)) => todo!()
//!     }
//! }
//!# fn view(&self, model: &Self::Model) {
//!#     unimplemented!()
//!# }
//!# }
//! ```
//!
//! Commands can be chained, allowing the outputs of the first effect to be used in constructing the second
//! effect. For example, the following code creates a new post, then fetches the full created post based
//! on a url read from the response to the creation request:
//!
//! ```
//!# use crux_core::Command;
//!# use crux_http::command::Http;
//!# use crux_core::render::render;
//!# use doctest_support::command::{Effect, Event, AnOperation, AnOperationOutput, Post};
//!# const API_URL: &str = "https://example.com/";
//!# let result = {
//! let cmd: Command<Effect, Event> =
//!     Http::post(API_URL)
//!         .body(serde_json::json!({"title":"New Post", "body":"Hello!"}))
//!         .expect_json::<Post>()
//!         .build()
//!         .then_request(|result| {
//!             let post = result.unwrap();
//!             let url = &post.body().unwrap().url;
//!
//!             Http::get(url).expect_json().build()
//!         })
//!         .then_send(Event::GotPost);
//!
//! // Run the http request concurrently with notifying the shell to render
//! Command::all([cmd, render()])
//!# };
//! ```
//!
//! The same can be done with the async API, if you need more complex orchestration that is
//! more naturally expressed in async rust
//!
//! ```
//! # use crux_core::Command;
//! # use crux_http::command::Http;
//! # use doctest_support::command::{Effect, Event, AnOperation, AnOperationOutput, Post};
//! # const API_URL: &str = "";
//! let cmd: Command<Effect, Event> = Command::new(|ctx| async move {
//!     let first = Http::post(API_URL)
//!         .body(serde_json::json!({"title":"New Post", "body":"Hello!"}))
//!         .expect_json::<Post>()
//!         .build()
//!         .into_future(ctx.clone())
//!         .await;
//!
//!     let post = first.unwrap();
//!     let url = &post.body().unwrap().url;
//!
//!     let second = Http::get(url).expect_json().build().into_future(ctx.clone()).await;
//!
//!     ctx.send_event(Event::GotPost(second));
//! });
//! ```
//!
//! In the async context, you can spawn additional concurrent tasks, which can, for example,
//! communicate with each other via channels, to enable more complex orchestrations, stateful
//! connection handling and other advanced uses.
//!
//! ```
//! # use crux_core::Command;
//! # use doctest_support::command::{Effect, Event, AnOperation};
//! let mut cmd: Command<Effect, Event> = Command::new(|ctx| async move {
//!     let (tx, rx) = async_channel::unbounded();
//!
//!     ctx.spawn(|ctx| async move {
//!         for i in 0..10u8 {
//!             let output = ctx.request_from_shell(AnOperation::One(i)).await;
//!             tx.send(output).await.unwrap();
//!         }
//!     });
//!
//!     ctx.spawn(|ctx| async move {
//!         while let Ok(value) = rx.recv().await {
//!             ctx.send_event(Event::Completed(value));
//!         }
//!         ctx.send_event(Event::Aborted);
//!     });
//! });
//! ```
//!
//! Commands can be cancelled, by calling [`Command::abort_handle`]
//! and then calling `abort` on the returned handle.
//!
//! ```
//! # use crux_core::Command;
//! # use doctest_support::command::{Effect, Event, AnOperation};
//! let mut cmd: Command<Effect, Event> = Command::all([
//!     Command::request_from_shell(AnOperation::One(1)).then_send(Event::Completed),
//!     Command::request_from_shell(AnOperation::Two(1)).then_send(Event::Completed),
//! ]);
//!
//! let handle = cmd.abort_handle();
//!
//! // Command is still running
//! assert!(!cmd.was_aborted());
//!
//! handle.abort();
//!
//! // Command is now finished
//! assert!(cmd.is_done());
//! // And was aborted
//! assert!(cmd.was_aborted());
//!
//! ```
//!
//! You can test that Commands yield the expected effects and events.
//! Commands can be tested in isolation by creating them explicitly
//! in a test, and then checking the effects and events they generated.
//! Or you can call your app's `update` function in a test, and perform
//! the same checks on the returned Command.
//!
//! ```
//! # use crux_http::{
//! #     command::Http,
//! #     protocol::{HttpRequest, HttpResponse, HttpResult},
//! #     testing::ResponseBuilder,
//! # };
//! # use doctest_support::command::{Effect, Event, Post};
//! const API_URL: &str = "https://example.com/api/posts";
//!
//! // Create a command to post a new Post to API_URL
//! // and then dispatch an event with the result
//! let mut cmd = Http::post(API_URL)
//!     .body(serde_json::json!({"title":"New Post", "body":"Hello!"}))
//!     .expect_json()
//!     .build()
//!     .then_send(Event::GotPost);
//!
//! // Check the effect is an HTTP request ...
//! let effect = cmd.effects().next().unwrap();
//! let Effect::Http(mut request) = effect else {
//!     panic!("Expected a HTTP effect")
//! };
//!
//! // ... and the request is a POST to API_URL
//! assert_eq!(
//!     &request.operation,
//!     &HttpRequest::post(API_URL)
//!         .header("content-type", "application/json")
//!         .body(r#"{"body":"Hello!","title":"New Post"}"#)
//!         .build()
//! );
//!
//! // Resolve the request with a successful response
//! let body = Post {
//!     url: API_URL.to_string(),
//!     title: "New Post".to_string(),
//!     body: "Hello!".to_string(),
//! };
//! request
//!     .resolve(HttpResult::Ok(HttpResponse::ok().json(&body).build()))
//!     .expect("Resolve should succeed");
//!
//! // Check the event is a GotPost event with the successful response
//! let actual = cmd.events().next().unwrap();
//! let expected = Event::GotPost(Ok(ResponseBuilder::ok().body(body).build()));
//! assert_eq!(actual, expected);
//!
//! assert!(cmd.is_done());
//! ```

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
use stream::CommandStreamExt as _;

pub use builder::{NotificationBuilder, RequestBuilder, StreamBuilder};
pub use context::CommandContext;
pub use stream::CommandOutput;

use crate::capability::Operation;
use crate::Request;

#[must_use = "Unused commands never execute. Return the command from your app's update function or combine it with other commands with Command::and or Command::all"]
pub struct Command<Effect, Event> {
    effects: Receiver<Effect>,
    events: Receiver<Event>,
    context: CommandContext<Effect, Event>,

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
    /// ([`and`](Command::and) and [`all`](Command::all)) to orchestrate them.
    ///
    /// The `create_task` closure receives a [`CommandContext`] that it can use to send shell requests,
    /// events back to the app, and to spawn additional tasks. The closure is expected to return a future
    /// which becomes the command's main asynchronous task.
    pub fn new<F, Fut>(create_task: F) -> Self
    where
        F: FnOnce(CommandContext<Effect, Event>) -> Fut,
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
        let (_, waker_receiver) = crossbeam_channel::unbounded();

        let context = context::CommandContext {
            effects: effect_sender,
            events: event_sender,
            tasks: spawn_sender,
        };

        let aborted: Arc<AtomicBool> = Default::default();
        let task = Task {
            finished: Default::default(),
            aborted: aborted.clone(),
            future: create_task(context.clone()).boxed(),
            join_handle_wakers: waker_receiver,
        };

        let mut tasks = Slab::with_capacity(1);
        let task_id = TaskId(tasks.insert(task));

        ready_sender
            .send(task_id)
            .expect("Could not make task ready, ready channel disconnected");

        Command {
            effects: effect_receiver,
            events: event_receiver,
            context,
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
        Command::new(|_ctx| futures::future::ready(()))
    }

    /// Create a command from another command with compatible `Effect` and `Event` types
    pub fn from<Ef, Ev>(subcmd: Command<Ef, Ev>) -> Self
    where
        Ef: Send + 'static + Into<Effect> + Unpin,
        Ev: Send + 'static + Into<Event> + Unpin,
        Effect: Unpin,
        Event: Unpin,
    {
        subcmd.map_effect(|ef| ef.into()).map_event(|ev| ev.into())
    }

    /// Turn the command into another command with compatible `Effect` and `Event` types
    pub fn into<Ef, Ev>(self) -> Command<Ef, Ev>
    where
        Ef: Send + 'static + Unpin,
        Ev: Send + 'static + Unpin,
        Effect: Unpin + Into<Ef>,
        Event: Unpin + Into<Ev>,
    {
        self.map_effect(|ef| ef.into()).map_event(|ev| ev.into())
    }

    /// Create a Command which dispatches an event and terminates. This is an alternative
    /// to calling `update` recursively. The only difference is that the two `update` calls
    /// will be visible to Crux and can show up in logs or any tooling. The trade-off is that
    /// the event is not guaranteed to dispatch instantly - another `update` call which is
    /// already scheduled may happen first.
    pub fn event(event: Event) -> Self {
        Command::new(|ctx| async move { ctx.send_event(event) })
    }

    /// Start a creation of a Command which sends a notification to the shell with a provided
    /// `operation`.
    ///
    /// Returns a [`NotificationBuilder`] which can be converted into a Command directly.
    ///
    /// In an async context, `NotificationBuilder` can be turned into a future that resolves to the
    /// operation output type.
    pub fn notify_shell<Op>(
        operation: Op,
    ) -> builder::NotificationBuilder<Effect, Event, impl Future<Output = ()>>
    where
        Op: Operation,
        Effect: From<Request<Op>>,
    {
        builder::NotificationBuilder::new(|ctx| async move { ctx.notify_shell(operation) })
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

    /// Run the effect state machine until it settles and return an iterator over the effects
    pub fn effects(&mut self) -> impl Iterator<Item = Effect> + '_ {
        self.run_until_settled();

        self.effects.try_iter()
    }

    /// Run the effect state machine until it settles and return an iterator over the events
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
    pub fn and(mut self, other: Self) -> Self
    where
        Effect: Unpin,
        Event: Unpin,
    {
        self.spawn(|ctx| other.host(ctx.effects, ctx.events).map(|_| ()));

        self
    }

    /// Create a command running a number of commands concurrently
    pub fn all<I>(commands: I) -> Self
    where
        I: IntoIterator<Item = Self>,
        Effect: Unpin,
        Event: Unpin,
    {
        let mut command = Command::done();

        for c in commands {
            command.spawn(|ctx| c.host(ctx.effects, ctx.events).map(|_| ()))
        }

        command
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

    /// Spawn an additional task on the command. The task will execute concurrently with
    /// existing tasks
    ///
    /// The `create_task` closure receives a [`CommandContext`] that it can use to send shell requests,
    /// events back to the app, and to spawn additional tasks. The closure is expected to return a future.
    pub fn spawn<F, Fut>(&mut self, create_task: F)
    where
        F: FnOnce(CommandContext<Effect, Event>) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.context.spawn(create_task);
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

impl<Effect, Event> FromIterator<Command<Effect, Event>> for Command<Effect, Event>
where
    Effect: Send + Unpin + 'static,
    Event: Send + Unpin + 'static,
{
    fn from_iter<I: IntoIterator<Item = Command<Effect, Event>>>(iter: I) -> Self {
        Command::all(iter)
    }
}

#[cfg(test)]
mod tests;
