//! Capabilities provide a user-friendly API to request side-effects from the shell.
//!
//! Typically, capabilities provide I/O and host API access. Capabilities are external to the
//! core Crux library. Some are part of the Crux core distribution, others are expected to be built by the
//! community. Apps can also build single-use capabilities where necessary.
//!
//! # Example use
//!
//! A typical use of a capability would look like the following:
//!
//! ```rust
//!# use url::Url;
//!# use crux_core::Command;
//!# const API_URL: &str = "";
//!# pub enum Event { Increment, Set(crux_http::Result<crux_http::Response<usize>>) }
//!# #[derive(crux_core::macros::Effect)]
//!# pub struct Capabilities {
//!#     pub render: crux_core::render::Render<Event>,
//!#     pub http: crux_http::Http<Event>,
//!# }
//!# #[derive(Default)] pub struct Model { count: usize }
//!# #[derive(Default)] pub struct App;
//!#
//!# impl crux_core::App for App {
//!#     type Event = Event;
//!#     type Model = Model;
//!#     type ViewModel = ();
//!#     type Capabilities = Capabilities;
//!#     type Effect = Effect;
//! fn update(&self, event: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) -> Command<Effect, Event> {
//!     match event {
//!         //...
//!         Event::Increment => {
//!             model.count += 1;
//!             caps.render.render(); // Render capability
//!
//!             let base = Url::parse(API_URL).unwrap();
//!             let url = base.join("/inc").unwrap();
//!             caps.http.post(url).expect_json().send(Event::Set); // HTTP client capability
//!         }
//!         Event::Set(_) => todo!(),
//!     }
//!     Command::done()
//! }
//!# fn view(&self, model: &Self::Model) {
//!#     unimplemented!()
//!# }
//!# }

//! ```
//!
//! Capabilities don't _perform_ side-effects themselves, they request them from the Shell. As a consequence
//! the capability calls within the `update` function **only queue up the requests**. The side-effects themselves
//! are performed concurrently and don't block the update function.
//!
//! In order to use a capability, the app needs to include it in its `Capabilities` associated type and `WithContext`
//! trait implementation (which can be provided by the `crux_core::macros::Effect` macro). For example:
//!
//! ```rust
//! mod root {
//!
//! // An app module which can be reused in different apps
//! mod my_app {
//!     use crux_core::{capability::CapabilityContext, App, render::{self, Render, RenderOperation}, Command, Request};
//!     use crux_core::macros::Effect;
//!     use serde::{Serialize, Deserialize};
//!
//!     #[derive(Default)]
//!     pub struct MyApp;
//!     #[derive(Serialize, Deserialize)]
//!     pub struct Event;
//!
//!     // The `Effect` derive macro generates an `Effect` type that is used by the
//!     // Shell to dispatch side-effect requests to the right capability implementation
//!     // (and, in some languages, checking that all necessary capabilities are implemented)
//!     #[derive(Effect)]
//!     pub struct Capabilities {
//!         pub render: Render<Event>
//!     }
//!
//!     impl App for MyApp {
//!         type Model = ();
//!         type Event = Event;
//!         type ViewModel = ();
//!         type Capabilities = Capabilities;
//!         type Effect = Effect;
//!
//!         fn update(&self, event: Event, model: &mut (), caps: &Capabilities) -> Command<Effect, Event> {
//!             render::render()
//!         }
//!
//!         fn view(&self, model: &()) {
//!             ()
//!         }
//!     }
//! }
//! }
//! ```
//!
//! # Implementing a capability
//!
//! Capabilities provide an interface to request side-effects. The interface has asynchronous semantics
//! with a form of callback. A typical capability call can look like this:
//!
//! ```rust,ignore
//! caps.ducks.get_in_a_row(10, Event::RowOfDucks)
//! ```
//!
//! The call above translates into "Get 10 ducks in a row and return them to me using the `RowOfDucks` event".
//! The capability's job is to translate this request into a serializable message and instruct the Shell to
//! do the duck herding and when it receives the ducks back, wrap them in the requested event and return it
//! to the app.
//!
//! We will refer to `get_in_row` in the above call as an _operation_, the `10` is an _input_, and the
//! `Event::RowOfDucks` is an event constructor - a function, which eventually receives the row of ducks
//! and returns a variant of the `Event` enum. Conveniently, enum tuple variants can be used as functions,
//! and so that will be the typical use.
//!
//! This is what the capability implementation could look like:
//!
//! ```rust
//! use crux_core::{
//!     capability::{CapabilityContext, Operation},
//! };
//! use crux_core::macros::Capability;
//! use serde::{Serialize, Deserialize};
//!
//! // A duck
//! #[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
//! struct Duck;
//!
//! // Operations that can be requested from the Shell
//! #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
//! enum DuckOperation {
//!     GetInARow(usize)
//! }
//!
//! // Respective outputs for those operations
//! #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
//! enum DuckOutput {
//!     GetInRow(Vec<Duck>)
//! }
//!
//! // Link the input and output type
//! impl Operation for DuckOperation {
//!     type Output = DuckOutput;
//! }
//!
//! // The capability. Context will provide the interface to the rest of the system.
//! #[derive(Capability)]
//! struct Ducks<Event> {
//!     context: CapabilityContext<DuckOperation, Event>
//! };
//!
//! impl<Event> Ducks<Event> {
//!     pub fn new(context: CapabilityContext<DuckOperation, Event>) -> Self {
//!         Self { context }
//!     }
//!
//!     pub fn get_in_a_row<F>(&self, number_of_ducks: usize, event: F)
//!     where
//!         Event: 'static,
//!         F: FnOnce(Vec<Duck>) -> Event + Send + 'static,
//!     {
//!         let ctx = self.context.clone();
//!         // Start a shell interaction
//!         self.context.spawn(async move {
//!             // Instruct Shell to get ducks in a row and await the ducks
//!             let ducks = ctx.request_from_shell(DuckOperation::GetInARow(number_of_ducks)).await;
//!
//!             // Unwrap the ducks and wrap them in the requested event
//!             // This will always succeed, as long as the Shell implementation is correct
//!             // and doesn't send the wrong output type back
//!             if let DuckOutput::GetInRow(ducks) = ducks {
//!                 // Queue an app update with the ducks event
//!                 ctx.update_app(event(ducks));
//!             }
//!         })
//!    }
//! }
//! ```
//!
//! The `self.context.spawn` API allows a multi-step transaction with the Shell to be performed by a capability
//! without involving the app, until the exchange has completed. During the exchange, one or more events can
//! be emitted (allowing a subscription or streaming like capability to be built).
//!
//! For Shell requests that have no output, you can use [`CapabilityContext::notify_shell`].
//!
//! `DuckOperation` and `DuckOutput` show how the set of operations can be extended. In simple capabilities,
//! with a single operation, these can be structs, or simpler types. For example, the HTTP capability works directly with
//! `HttpRequest` and `HttpResponse`.

pub(crate) mod channel;

mod executor;
mod shell_request;
mod shell_stream;

use futures::{Future, Stream, StreamExt as _};
use serde::de::DeserializeOwned;
use std::sync::Arc;

pub(crate) use channel::channel;
pub(crate) use executor::{executor_and_spawner, QueuingExecutor};

use crate::{command::CommandOutput, Command, Request};
use channel::Sender;

/// Operation trait links together input and output of a side-effect.
///
/// You implement `Operation` on the payload sent by the capability to the shell using [`CapabilityContext::request_from_shell`].
///
/// For example (from `crux_http`):
///
/// ```rust,ignore
/// impl Operation for HttpRequest {
///     type Output = HttpResponse;
/// }
/// ```
pub trait Operation:
    serde::Serialize + serde::de::DeserializeOwned + Clone + PartialEq + Send + 'static
{
    /// `Output` assigns the type this request results in.
    type Output: serde::de::DeserializeOwned + Send + Unpin + 'static;

    #[cfg(feature = "typegen")]
    fn register_types(generator: &mut crate::typegen::TypeGen) -> crate::typegen::Result {
        generator.register_type::<Self>()?;
        generator.register_type::<Self::Output>()?;
        Ok(())
    }
}

/// A type that can be used as a capability operation, but which will never be sent to the shell.
/// This type is useful for capabilities that don't request effects.
/// For example, you can use this type as the Operation for a
/// capability that just composes other capabilities.
///
/// e.g.
/// ```rust
/// # use crux_core::capability::{CapabilityContext, Never};
/// # use crux_core::macros::Capability;
/// #[derive(Capability)]
/// pub struct Compose<E> {
///     context: CapabilityContext<Never, E>,
/// }
/// # impl<E> Compose<E> {
/// #     pub fn new(context: CapabilityContext<Never, E>) -> Self {
/// #         Self { context }
/// #     }
/// # }
///
/// ```
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Never {}

/// Implement `Operation` for `Never` to allow using it as a capability operation.
impl Operation for Never {
    type Output = ();
}

/// Implement the `Capability` trait for your capability. This will allow
/// mapping events when composing apps from submodules.
///
/// Note that this implementation can be generated by the `crux_core::macros::Capability` derive macro.
///
/// Example:
///
/// ```rust
/// # use crux_core::{Capability, capability::{CapabilityContext, Operation}};
/// # pub struct Http<Ev> {
/// #     context: CapabilityContext<HttpRequest, Ev>,
/// # }
/// # #[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)] pub struct HttpRequest;
/// # impl Operation for HttpRequest {
/// #     type Output = ();
/// # }
/// # impl<Ev> Http<Ev> where Ev: 'static, {
/// #     pub fn new(context: CapabilityContext<HttpRequest, Ev>) -> Self {
/// #         Self { context }
/// #     }
/// # }
/// impl<Ev> Capability<Ev> for Http<Ev> {
///     type Operation = HttpRequest;
///     type MappedSelf<MappedEv> = Http<MappedEv>;
///
///     fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
///     where
///         F: Fn(NewEvent) -> Ev + Send + Sync + 'static,
///         Ev: 'static,
///         NewEvent: 'static,
///     {
///         Http::new(self.context.map_event(f))
///     }
/// }
/// ```
pub trait Capability<Ev> {
    type Operation: Operation + DeserializeOwned;

    type MappedSelf<MappedEv>;

    fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
    where
        F: Fn(NewEv) -> Ev + Send + Sync + 'static,
        Ev: 'static,
        NewEv: 'static + Send;
}

/// Allows Crux to construct app's set of required capabilities, providing context
/// they can then use to request effects and dispatch events.
///
/// `new_with_context` is called by Crux and should return an instance of the app's `Capabilities` type with
/// all capabilities constructed with context passed in. Use `Context::specialize` to
/// create an appropriate context instance with the effect constructor which should
/// wrap the requested operations.
///
/// Note that this implementation can be generated by the derive macro `crux_core::macros::Effect`.
///
/// ```rust
/// # #[derive(Default)]
/// # struct App;
/// # pub enum Event {}
/// # #[allow(dead_code)]
/// # pub struct Capabilities {
/// #     http: crux_http::Http<Event>,
/// #     render: crux_core::render::Render<Event>,
/// # }
/// # pub enum Effect {
/// #     Http(crux_core::Request<<crux_http::Http<Event> as crux_core::capability::Capability<Event>>::Operation>),
/// #     Render(crux_core::Request<<crux_core::render::Render<Event> as crux_core::capability::Capability<Event>>::Operation>),
/// # }
/// # #[derive(serde::Serialize)]
/// # pub enum EffectFfi {
/// #     Http(<crux_http::Http<Event> as crux_core::capability::Capability<Event>>::Operation),
/// #     Render(<crux_core::render::Render<Event> as crux_core::capability::Capability<Event>>::Operation),
/// # }
/// # impl crux_core::App for App {
/// #     type Event = Event;
/// #     type Model = ();
/// #     type ViewModel = ();
/// #     type Capabilities = Capabilities;
/// #     type Effect = Effect;
/// #     fn update(&self, _event: Self::Event, _model: &mut Self::Model, _caps: &Self::Capabilities) -> crux_core::Command<Effect, Event> {
/// #         unimplemented!()
/// #     }
/// #     fn view(&self, _model: &Self::Model) -> Self::ViewModel {
/// #         unimplemented!()
/// #     }
/// # }
/// # impl crux_core::Effect for Effect {
/// #     type Ffi = EffectFfi;
/// #     fn serialize(self) -> (Self::Ffi, crux_core::bridge::ResolveSerialized) {
/// #         match self {
/// #             Effect::Http(request) => request.serialize(EffectFfi::Http),
/// #             Effect::Render(request) => request.serialize(EffectFfi::Render),
/// #         }
/// #     }
/// # }
/// impl crux_core::WithContext<Event, Effect> for Capabilities {
///     fn new_with_context(
///         context: crux_core::capability::ProtoContext<Effect, Event>,
///     ) -> Capabilities {
///         Capabilities {
///             http: crux_http::Http::new(context.specialize(Effect::Http)),
///             render: crux_core::render::Render::new(context.specialize(Effect::Render)),
///         }
///     }
/// }
/// ```
pub trait WithContext<Ev, Ef> {
    fn new_with_context(context: ProtoContext<Ef, Ev>) -> Self;
}

impl<Event, Effect> WithContext<Event, Effect> for () {
    fn new_with_context(_context: ProtoContext<Effect, Event>) -> Self {}
}

/// An interface for capabilities to interact with the app and the shell.
///
/// To use [`update_app`](CapabilityContext::update_app), [`notify_shell`](CapabilityContext::notify_shell)
/// or [`request_from_shell`](CapabilityContext::request_from_shell), spawn a task first.
///
/// For example (from `crux_time`)
///
/// ```rust
/// # #[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)] pub struct TimeRequest;
/// # #[derive(Clone, serde::Deserialize)] pub struct TimeResponse(pub String);
/// # impl crux_core::capability::Operation for TimeRequest {
/// #     type Output = TimeResponse;
/// # }
/// # pub struct Time<Ev> {
/// #     context: crux_core::capability::CapabilityContext<TimeRequest, Ev>,
/// # }
/// # impl<Ev> Time<Ev> where Ev: 'static, {
/// #     pub fn new(context: crux_core::capability::CapabilityContext<TimeRequest, Ev>) -> Self {
/// #         Self { context }
/// #     }
///
/// pub fn get<F>(&self, callback: F)
/// where
///     F: FnOnce(TimeResponse) -> Ev + Send + Sync + 'static,
/// {
///     let ctx = self.context.clone();
///     self.context.spawn(async move {
///         let response = ctx.request_from_shell(TimeRequest).await;
///
///         ctx.update_app(callback(response));
///     });
/// }
/// # }
/// ```
///
// used in docs/internals/runtime.md
// ANCHOR: capability_context
pub struct CapabilityContext<Op, Event>
where
    Op: Operation,
{
    inner: std::sync::Arc<ContextInner<Op, Event>>,
}

struct ContextInner<Op, Event>
where
    Op: Operation,
{
    shell_channel: Sender<Request<Op>>,
    app_channel: Sender<Event>,
    spawner: executor::Spawner,
}
// ANCHOR_END: capability_context

/// Initial version of capability Context which has not yet been specialized to a chosen capability
pub struct ProtoContext<Eff, Event> {
    shell_channel: Sender<Eff>,
    app_channel: Sender<Event>,
    spawner: executor::Spawner,
}

impl<Eff, Event> Clone for ProtoContext<Eff, Event> {
    fn clone(&self) -> Self {
        Self {
            shell_channel: self.shell_channel.clone(),
            app_channel: self.app_channel.clone(),
            spawner: self.spawner.clone(),
        }
    }
}

// CommandSpawner is a temporary bridge between the channel type used by the Command and the channel type
// used by the core. Once the old capability support is removed, we should be able to remove this in favour
// of the Command's ability to be hosted on a pair of channels
pub(crate) struct CommandSpawner<Effect, Event> {
    context: ProtoContext<Effect, Event>,
}

impl<Effect, Event> CommandSpawner<Effect, Event> {
    pub(crate) fn new(context: ProtoContext<Effect, Event>) -> Self {
        Self { context }
    }

    pub(crate) fn spawn(&self, mut command: Command<Effect, Event>)
    where
        Command<Effect, Event>: Stream<Item = CommandOutput<Effect, Event>>,
        Effect: Unpin + Send + 'static,
        Event: Unpin + Send + 'static,
    {
        self.context.spawner.spawn({
            let context = self.context.clone();

            async move {
                while let Some(output) = command.next().await {
                    match output {
                        CommandOutput::Effect(effect) => context.shell_channel.send(effect),
                        CommandOutput::Event(event) => context.app_channel.send(event),
                    }
                }
            }
        });
    }
}

impl<Op, Ev> Clone for CapabilityContext<Op, Ev>
where
    Op: Operation,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<Eff, Ev> ProtoContext<Eff, Ev>
where
    Ev: 'static,
    Eff: 'static,
{
    pub(crate) fn new(
        shell_channel: Sender<Eff>,
        app_channel: Sender<Ev>,
        spawner: executor::Spawner,
    ) -> Self {
        Self {
            shell_channel,
            app_channel,
            spawner,
        }
    }

    /// Specialize the CapabilityContext to a specific capability, wrapping its operations into
    /// an Effect `Ef`. The `func` argument will typically be an Effect variant constructor, but
    /// can be any function taking the capability's operation type and returning
    /// the effect type.
    ///
    /// This will likely only be called from the implementation of [`WithContext`]
    /// for the app's `Capabilities` type. You should not need to call this function directly.
    pub fn specialize<Op, F>(&self, func: F) -> CapabilityContext<Op, Ev>
    where
        F: Fn(Request<Op>) -> Eff + Sync + Send + Copy + 'static,
        Op: Operation,
    {
        CapabilityContext::new(
            self.shell_channel.map_input(func),
            self.app_channel.clone(),
            self.spawner.clone(),
        )
    }
}

impl<Op, Ev> CapabilityContext<Op, Ev>
where
    Op: Operation,
    Ev: 'static,
{
    pub(crate) fn new(
        shell_channel: Sender<Request<Op>>,
        app_channel: Sender<Ev>,
        spawner: executor::Spawner,
    ) -> Self {
        let inner = Arc::new(ContextInner {
            shell_channel,
            app_channel,
            spawner,
        });

        CapabilityContext { inner }
    }

    /// Spawn a task to do the asynchronous work. Within the task, async code
    /// can be used to interact with the Shell and the App.
    pub fn spawn(&self, f: impl Future<Output = ()> + 'static + Send) {
        self.inner.spawner.spawn(f);
    }

    /// Send an effect request to the shell in a fire and forget fashion. The
    /// provided `operation` does not expect anything to be returned back.
    pub async fn notify_shell(&self, operation: Op) {
        // This function might look like it doesn't need to be async but
        // it's important that it is.  It forces all capabilities to
        // spawn onto the executor which keeps the ordering of effects
        // consistent with their function calls.
        self.inner
            .shell_channel
            .send(Request::resolves_never(operation));
    }

    /// Send an event to the app. The event will be processed on the next
    /// run of the update loop. You can call `update_app` several times,
    /// the events will be queued up and processed sequentially after your
    /// async task either `await`s or finishes.
    pub fn update_app(&self, event: Ev) {
        self.inner.app_channel.send(event);
    }

    /// Transform the CapabilityContext into one which uses the provided function to
    /// map each event dispatched with `update_app` to a different event type.
    ///
    /// This is useful when composing apps from modules to wrap a submodule's
    /// event type with a specific variant of the parent module's event, so it can
    /// be forwarded to the submodule when received.
    ///
    /// In a typical case you would implement `From` on the submodule's `Capabilities` type
    ///
    /// ```rust
    /// # use crux_core::{Capability, Command};
    /// # #[derive(Default)]
    /// # struct App;
    /// # pub enum Event {
    /// #     Submodule(child::Event),
    /// # }
    /// # #[derive(crux_core::macros::Effect)]
    /// # pub struct Capabilities {
    /// #     some_capability: crux_time::Time<Event>,
    /// #     render: crux_core::render::Render<Event>,
    /// # }
    /// # impl crux_core::App for App {
    /// #     type Event = Event;
    /// #     type Model = ();
    /// #     type ViewModel = ();
    /// #     type Capabilities = Capabilities;
    /// #     type Effect = Effect;
    /// #     fn update(
    /// #         &self,
    /// #         _event: Self::Event,
    /// #         _model: &mut Self::Model,
    /// #         _caps: &Self::Capabilities,
    /// #     ) -> Command<Effect, Event> {
    /// #         unimplemented!()
    /// #     }
    /// #     fn view(&self, _model: &Self::Model) -> Self::ViewModel {
    /// #         unimplemented!()
    /// #     }
    /// # }
    ///impl From<&Capabilities> for child::Capabilities {
    ///    fn from(incoming: &Capabilities) -> Self {
    ///        child::Capabilities {
    ///            some_capability: incoming.some_capability.map_event(Event::Submodule),
    ///            render: incoming.render.map_event(Event::Submodule),
    ///        }
    ///    }
    ///}
    /// # mod child {
    /// #     #[derive(Default)]
    /// #     struct App;
    /// #     pub struct Event;
    /// #     #[derive(crux_core::macros::Effect)]
    /// #     pub struct Capabilities {
    /// #         pub some_capability: crux_time::Time<Event>,
    /// #         pub render: crux_core::render::Render<Event>,
    /// #     }
    /// #     impl crux_core::App for App {
    /// #         type Event = Event;
    /// #         type Model = ();
    /// #         type ViewModel = ();
    /// #         type Capabilities = Capabilities;
    /// #         type Effect = Effect;
    /// #         fn update(
    /// #             &self,
    /// #             _event: Self::Event,
    /// #             _model: &mut Self::Model,
    /// #             _caps: &Self::Capabilities,
    /// #         ) -> crux_core::Command<Effect, Event> {
    /// #             unimplemented!()
    /// #         }
    /// #         fn view(&self, _model: &Self::Model) -> Self::ViewModel {
    /// #             unimplemented!()
    /// #         }
    /// #     }
    /// # }
    /// ```
    ///
    /// in the parent module's `update` function, you can then call `.into()` on the
    /// capabilities, before passing them down to the submodule.
    pub fn map_event<NewEv, F>(&self, func: F) -> CapabilityContext<Op, NewEv>
    where
        F: Fn(NewEv) -> Ev + Sync + Send + 'static,
        NewEv: 'static,
    {
        CapabilityContext::new(
            self.inner.shell_channel.clone(),
            self.inner.app_channel.map_input(func),
            self.inner.spawner.clone(),
        )
    }

    pub(crate) fn send_request(&self, request: Request<Op>) {
        self.inner.shell_channel.send(request);
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use static_assertions::assert_impl_all;

    use super::*;

    #[allow(dead_code)]
    enum Effect {}

    #[allow(dead_code)]
    enum Event {}

    #[derive(PartialEq, Clone, Serialize, Deserialize)]
    struct Op {}

    impl Operation for Op {
        type Output = ();
    }

    assert_impl_all!(ProtoContext<Effect, Event>: Send, Sync);
    assert_impl_all!(CapabilityContext<Op, Event>: Send, Sync);
}
