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
//!# const API_URL: &str = "";
//!# pub enum Event { Increment, Set(crux_http::Result<crux_http::Response<usize>>) }
//!# #[derive(crux_core::macros::Effect)]
//!# pub struct Capabilities {
//!#     pub render: crux_core::render::Render,
//!#     pub http: crux_http::Http,
//!# }
//!# #[derive(Default)] pub struct Model { count: usize }
//!# #[derive(Default)] pub struct App;
//!#
//!# impl crux_core::App for App {
//!#     type Event = Event;
//!#     type Model = Model;
//!#     type ViewModel = ();
//!#     type Capabilities = Capabilities;
//! fn update(&self, event: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) -> crux_core::Command<Self::Event> {
//!     match event {
//!         //...
//!         Event::Increment => {
//!             model.count += 1;
//!
//!             let base = Url::parse(API_URL).unwrap();
//!             let url = base.join("/inc").unwrap();
//!             let cmd1 = caps.http.post(url).expect_json().send_and_respond(Event::Set); // HTTP client capability
//!             let cmd2 = caps.render.render(); // Render capability
//!             cmd1.join(cmd2)
//!         }
//!         Event::Set(_) => todo!(),
//!     }
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
//!     use crux_core::{capability::CapabilityContext, App, Command, render::Render};
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
//!         pub render: Render
//!     }
//!
//!     impl App for MyApp {
//!         type Model = ();
//!         type Event = Event;
//!         type ViewModel = ();
//!         type Capabilities = Capabilities;
//!
//!         fn update(&self, event: Event, model: &mut (), caps: &Capabilities) -> Command<Event> {
//!             caps.render.render()
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
//!     Command,
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
//! struct Ducks {
//!     context: CapabilityContext<DuckOperation>
//! };
//!
//! impl Ducks {
//!     pub fn new(context: CapabilityContext<DuckOperation>) -> Self {
//!         Self { context }
//!     }
//!
//!     pub fn get_in_a_row<F, Event>(&self, number_of_ducks: usize, event: F) -> Command<Event>
//!     where
//!         Event: 'static,
//!         F: FnOnce(Vec<Duck>) -> Event + Send + 'static,
//!     {
//!         let ctx = self.context.clone();
//!         // Start a shell interaction
//!         Command::effect(async move {
//!             // Instruct Shell to get ducks in a row and await the ducks
//!             let ducks = ctx.request_from_shell(DuckOperation::GetInARow(number_of_ducks)).await;
//!
//!             // Unwrap the ducks and wrap them in the requested event
//!             // This will always succeed, as long as the Shell implementation is correct
//!             // and doesn't send the wrong output type back
//!             if let DuckOutput::GetInRow(ducks) = ducks {
//!                 // Queue an app update with the ducks event
//!                 Command::event(event(ducks))
//!             } else {
//!                 Command::none()
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

use serde::de::DeserializeOwned;

pub(crate) use channel::channel;
pub(crate) use executor::QueuingExecutor;

use crate::Request;
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
pub trait Operation: serde::Serialize + PartialEq + Send + 'static {
    /// `Output` assigns the type this request results in.
    type Output: serde::de::DeserializeOwned + Send + 'static;
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
/// pub struct Compose {
///     context: CapabilityContext<Never>,
/// }
/// # impl Compose {
/// #     pub fn new(context: CapabilityContext<Never>) -> Self {
/// #         Self { context }
/// #     }
/// # }
///
/// ```
#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
/// # pub struct Http {
/// #     context: CapabilityContext<HttpRequest>,
/// # }
/// # #[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq)] pub struct HttpRequest;
/// # impl Operation for HttpRequest {
/// #     type Output = ();
/// # }
/// # impl Http {
/// #     pub fn new(context: CapabilityContext<HttpRequest>) -> Self {
/// #         Self { context }
/// #     }
/// # }
/// impl Capability for Http {
///     type Operation = HttpRequest;
/// }
/// ```
pub trait Capability {
    type Operation: Operation + DeserializeOwned;

    #[cfg(feature = "typegen")]
    fn register_types(generator: &mut crate::typegen::TypeGen) -> crate::typegen::Result {
        generator.register_type::<Self::Operation>()?;
        generator.register_type::<<Self::Operation as Operation>::Output>()?;
        Ok(())
    }
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
/// #     http: crux_http::Http,
/// #     render: crux_core::render::Render,
/// # }
/// # pub enum Effect {
/// #     Http(crux_core::Request<<crux_http::Http as crux_core::capability::Capability>::Operation>),
/// #     Render(crux_core::Request<<crux_core::render::Render as crux_core::capability::Capability>::Operation>),
/// # }
/// # #[derive(serde::Serialize)]
/// # pub enum EffectFfi {
/// #     Http(<crux_http::Http as crux_core::capability::Capability>::Operation),
/// #     Render(<crux_core::render::Render as crux_core::capability::Capability>::Operation),
/// # }
/// # impl crux_core::App for App {
/// #     type Event = Event;
/// #     type Model = ();
/// #     type ViewModel = ();
/// #     type Capabilities = Capabilities;
/// #     fn update(&self, _event: Self::Event, _model: &mut Self::Model, _caps: &Self::Capabilities) -> crux_core::Command<Event> {
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
/// impl crux_core::WithContext<Effect> for Capabilities {
///     fn new_with_context(
///         context: crux_core::capability::ProtoContext<Effect>,
///     ) -> Capabilities {
///         Capabilities {
///             http: crux_http::Http::new(context.specialize(Effect::Http)),
///             render: crux_core::render::Render::new(context.specialize(Effect::Render)),
///         }
///     }
/// }
/// ```
pub trait WithContext<Ef> {
    fn new_with_context(context: ProtoContext<Ef>) -> Self;
}

/// An interface for capabilities to interact with the app and the shell.
///
/// To use [`update_app`](CapabilityContext::update_app), [`notify_shell`](CapabilityContext::notify_shell)
/// or [`request_from_shell`](CapabilityContext::request_from_shell), spawn a task first.
///
/// For example (from `crux_time`)
///
/// ```rust
/// # use crux_core::Command;
/// # #[derive(PartialEq,serde::Serialize)]pub struct TimeRequest;
/// # #[derive(serde::Deserialize)]pub struct TimeResponse(pub String);
/// # impl crux_core::capability::Operation for TimeRequest {
/// #     type Output = TimeResponse;
/// # }
/// # pub struct Time {
/// #     context: crux_core::capability::CapabilityContext<TimeRequest>,
/// # }
/// # impl Time {
/// #     pub fn new(context: crux_core::capability::CapabilityContext<TimeRequest>) -> Self {
/// #         Self { context }
/// #     }
///
/// pub fn get<F, Ev>(&self, callback: F) -> Command<Ev>
/// where
///     F: FnOnce(TimeResponse) -> Ev + Send + Sync + 'static,
/// {
///     let ctx = self.context.clone();
///     Command::effect(async move {
///         let response = ctx.request_from_shell(TimeRequest).await;
///         Command::event(callback(response))
///     })
/// }
/// # }
/// ```
///
// used in docs/internals/runtime.md
// ANCHOR: capability_context
pub struct CapabilityContext<Op>
where
    Op: Operation,
{
    shell_channel: Sender<Request<Op>>,
}

impl<Op> Clone for CapabilityContext<Op>
where
    Op: Operation,
{
    fn clone(&self) -> Self {
        Self {
            shell_channel: self.shell_channel.clone(),
        }
    }
}

// ANCHOR_END: capability_context

/// Initial version of capability Context which has not yet been specialized to a chosen capability
pub struct ProtoContext<Eff> {
    shell_channel: Sender<Eff>,
}

impl<Eff> ProtoContext<Eff>
where
    Eff: 'static,
{
    pub(crate) fn new(shell_channel: Sender<Eff>) -> Self {
        Self { shell_channel }
    }

    /// Specialize the CapabilityContext to a specific capability, wrapping its operations into
    /// an Effect `Ef`. The `func` argument will typically be an Effect variant constructor, but
    /// can be any function taking the capability's operation type and returning
    /// the effect type.
    ///
    /// This will likely only be called from the implementation of [`WithContext`]
    /// for the app's `Capabilities` type. You should not need to call this function directly.
    pub fn specialize<Op, F>(&self, func: F) -> CapabilityContext<Op>
    where
        F: Fn(Request<Op>) -> Eff + Sync + Send + Copy + 'static,
        Op: Operation,
    {
        CapabilityContext::new(self.shell_channel.map_input(func))
    }
}

impl<Op> CapabilityContext<Op>
where
    Op: Operation,
{
    pub(crate) fn new(shell_channel: Sender<Request<Op>>) -> Self {
        Self { shell_channel }
    }

    /// Send an effect request to the shell in a fire and forget fashion. The
    /// provided `operation` does not expect anything to be returned back.
    pub async fn notify_shell(&self, operation: Op) {
        // This function might look like it doesn't need to be async but
        // it's important that it is.  It forces all capabilities to
        // spawn onto the executor which keeps the ordering of effects
        // consistent with their function calls.
        self.shell_channel.send(Request::resolves_never(operation));
    }

    pub(crate) fn send_request(&self, request: Request<Op>) {
        self.shell_channel.send(request);
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use static_assertions::assert_impl_all;

    use super::*;

    #[allow(dead_code)]
    enum Effect {}

    #[allow(dead_code)]
    enum Event {}

    #[derive(PartialEq, Serialize)]
    struct Op {}

    impl Operation for Op {
        type Output = ();
    }

    assert_impl_all!(ProtoContext<Effect>: Send, Sync);
    assert_impl_all!(CapabilityContext<Op>: Send, Sync);
}
