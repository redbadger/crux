//! ## DEPRECATED
//!
//! Capabilities are the legacy interface to side-effects, and this module will be removed in a future version
//! of crux. If you're starting a new app, you should use the [`command`](crate::command) API.
//!
//! For more help migrating from Capabilities to Commands, see [the documentation book](https://redbadger.github.io/crux/guide/effects.html#migrating-from-previous-versions-of-crux)
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
use futures::{Stream, StreamExt as _};

pub(crate) use channel::channel;
pub(crate) use executor::{QueuingExecutor, executor_and_spawner};

#[cfg(feature = "facet_typegen")]
use crate::type_generation::facet::TypeGenError;
use crate::{Command, command::CommandOutput};
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
pub trait Operation: Send + 'static {
    /// `Output` assigns the type this request results in.
    type Output: Send + Unpin + 'static;

    #[cfg(feature = "typegen")]
    #[allow(clippy::missing_errors_doc)]
    fn register_types(
        generator: &mut crate::type_generation::serde::TypeGen,
    ) -> crate::type_generation::serde::Result
    where
        Self: serde::Serialize + for<'de> serde::de::Deserialize<'de>,
        Self::Output: for<'de> serde::de::Deserialize<'de>,
    {
        generator.register_type::<Self>()?;
        generator.register_type::<Self::Output>()?;
        Ok(())
    }

    #[cfg(feature = "facet_typegen")]
    #[allow(clippy::missing_errors_doc)]
    fn register_types_facet<'a>(
        generator: &mut crate::type_generation::facet::TypeRegistry,
    ) -> Result<&mut crate::type_generation::facet::TypeRegistry, TypeGenError>
    where
        Self: facet::Facet<'a> + serde::Serialize + for<'de> serde::de::Deserialize<'de>,
        <Self as Operation>::Output: facet::Facet<'a> + for<'de> serde::de::Deserialize<'de>,
    {
        generator
            .register_type::<Self>()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?
            .register_type::<Self::Output>()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;

        Ok(generator)
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
}
