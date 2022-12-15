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
//! ```rust,ignore
//! fn update(&self, event: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) {
//!     match event {
//!         ...         
//!         Event::Increment => {
//!             model.count += 1;
//!             caps.render.render(); // Render capability
//!
//!             let base = Url::parse(API_URL).unwrap();
//!             let url = base.join("/inc");
//!             caps.http.post(url.unwrap(), Event::Set) // HTTP client capability
//!         }
//!     }
//! }
//! ```
//!
//! Capabilities don't _perform_ side-effects themselves, they request them from the Shell. As a consequence
//! The capability calls within the `update` function **only queue up the requests**. The side-effects themselves
//! are performed concurrently and don't block the update function.
//!
//! In order to use a capability, the app needs to include it in its `Capabilities` associated type and `WithContext`
//! trait implementation. For example:
//!
//! ```rust
//! mod root {
//!
//! // An app module which can be reused in different apps
//! mod my_app {
//!     use crux_core::App;
//!     use crux_core::render::{Render};
//!     
//!     #[derive(Default)]
//!     pub struct MyApp;
//!     pub struct Event;
//!    
//!     pub struct Capabilities {
//!         pub render: Render<Event>
//!     }
//!     
//!     impl App for MyApp {
//!         type Model = ();
//!         type Event = Event;
//!         type ViewModel = ();
//!         type Capabilities = Capabilities;
//!    
//!         fn update(&self, event: Event, model: &mut (), caps: &Capabilities) {
//!             caps.render.render();
//!         }
//!     
//!         fn view(&self, model: &()) {
//!             ()    
//!         }
//!     }
//! }
//!
//! // Specific app core using the my_app module
//! mod core {
//!     use serde::{Serialize, Deserialize};
//!     use crux_core::capability::{CapabilityContext};
//!     use crux_core::render::{Render};
//!     use super::my_app::{MyApp, Event, Capabilities};
//!
//!     #[derive(Serialize, Deserialize)]
//!     enum Effect {
//!         Render
//!     }
//!    
//!     impl crux_core::WithContext<MyApp, Effect> for Capabilities {
//!        fn new_with_context(context: CapabilityContext<Effect, Event>) -> Capabilities {
//!            Capabilities {
//!                render: Render::new(context.with_effect(|_| Effect::Render)),
//!            }
//!        }
//!     }
//! }
//! }
//! ```
//!
//! This links the (reusable) capabilities with the app-specific `Effect` type, used by the Shell to dispatch
//! side-effect requests to the right capability implementation (and, in some languages, checking that all necessary
//! capabilities are implemented).
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
//! The capability's job is to translate this request into serialisable message and instruct the Shell to
//! do the duck herding and when it receives the ducks back, wrap them in the requested event and return
//! to the app.
//!
//! We will refer to `get_in_row` in the above call as an _operation_, the `10` is an _input_, and the
//! `Event::RowOfDucks` is an event constructor - a function, which eventually receives the row of ducks
//! and returns a variant of the `Event` enum. Conveniently, enum variants are functions, and so that
//! will be the typical use.
//!
//! This is what the capability implementation could look like:
//!
//! ```rust
//! use crux_core::{
//!     capability::{CapabilityContext, Operation},
//!     Capability,
//! };
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
//! struct Ducks<Event> {
//!     context: CapabilityContext<DuckOperation, Event>
//! };
//!
//! // Basic empty implementation of the capability
//! impl<Event> Ducks<Event> {
//!     pub fn new(context: CapabilityContext<DuckOperation, Event>) -> Self {
//!         Self { context }
//!     }
//!
//!     pub fn get_in_a_row<F>(&self, number_of_ducks: usize, event: F)
//!     where
//!         Event: 'static,
//!         F: Fn(Vec<Duck>) -> Event + Send + 'static,
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

use std::sync::Arc;

use futures::Future;

use crate::{channels::Sender, Step};

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
pub trait Operation: serde::Serialize + Send + 'static {
    /// `Output` assigns the type this request results in.
    type Output: serde::de::DeserializeOwned + Send + 'static;
}

/// Implement `Capability` for your capability. This will allow
/// mapping events when composing apps from submodules.
///
/// In the future this implementation will likely be provided by a derive macro.
///
/// Example:
///
/// ```rust,ignore
/// impl<Ev> Capability<Ev> for Http<Ev> {
///     type MappedSelf<MappedEv> = Http<MappedEv>;
///
///     fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
///     where
///         F: Fn(NewEvent) -> Ef + Send + Sync + Copy + 'static,
///         Ev: 'static,
///         NewEvent: 'static,
///     {
///         Http::new(self.context.map_event(f))
///     }
/// }
/// ```
pub trait Capability<Ev> {
    type MappedSelf<MappedEv>;

    fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
    where
        F: Fn(NewEvent) -> Ev + Send + Sync + Copy + 'static,
        Ev: 'static,
        NewEvent: 'static;
}

/// Allows Crux to construct app's set of required capabilities, providing context
/// they can then use to request effects and dispatch events.
///
/// `new_with_context` is called by Crux and should return an instance of the app's `Capabilities` type with
/// all capabilities constructed with context passed in. Use `Context::with_effect` to
/// create an appropriate context instance with the effect constructor which should
/// wrap the requested operations. (See example above)
///
/// ```rust,ignore
/// impl crux_core::WithContext<App, Effect> for Capabilities {
///     fn new_with_context(context: CapabilityContext<Effect, Event>) -> Capabilities {
///         Capabilities {
///             http: Http::new(context.with_effect(Effect::Http)),
///             render: Render::new(context.with_effect(|_| Effect::Render)),
///         }
///     }
/// }
/// ```
pub trait WithContext<App, Ef>
where
    App: crate::App,
{
    fn new_with_context(context: CapabilityContext<Ef, App::Event>) -> App::Capabilities;
}

/// An interface for capabilites to interact with the app and the shell.
///
/// To use [`update_app`](CapabilityContext::update_app), [`notify_shell`](CapabilityContext::notify_shell)
/// or [`request_from_shell`](CapabilityContext::request_from_shell), spawn a task first.
///
/// For example (from `crux_time`)
///
/// ```rust,ignore
/// pub fn get<F>(&self, callback: F)
/// where
///     F: Fn(TimeResponse) -> Ev + Send + Sync + 'static,
/// {
///     let ctx = self.context.clone();
///     self.context.spawn(async move {
///         let response = ctx.request_from_shell(TimeRequest).await;
///
///         ctx.update_app(callback(response));
///     });
/// }
/// ```
///
pub struct CapabilityContext<Ef, Event> {
    inner: std::sync::Arc<ContextInner<Ef, Event>>,
}

struct ContextInner<Ef, Event> {
    steps: Sender<Step<Ef>>,
    events: Sender<Event>,
    spawner: crate::executor::Spawner,
}

impl<Ef, Ev> Clone for CapabilityContext<Ef, Ev> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T, Ev> CapabilityContext<T, Ev>
where
    T: Send + 'static,
    Ev: 'static,
{
    pub(crate) fn new(
        steps: Sender<Step<T>>,
        events: Sender<Ev>,
        spawner: crate::executor::Spawner,
    ) -> Self {
        let inner = Arc::new(ContextInner {
            steps,
            events,
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
    pub async fn notify_shell(&self, operation: T) {
        // This function might look like it doesn't need to be async but
        // it's important that it is.  It forces all capabilities to
        // spawn onto the executor which keeps the ordering of effects
        // consistent with their function calls.
        self.inner.steps.send(Step::once(operation));
    }

    /// Send an event to the app. The event will be processed a the next
    /// run of the update loop. You can call `update_app` several times,
    /// the events will be queued up and processed sequentially after your
    /// async task either `await`s or finishes.
    pub fn update_app(&self, event: Ev) {
        self.inner.events.send(event);
    }

    /// Construct a CapabilityContext with a specific effect wrapper. The
    /// `func` argument will typically be an enum variant constructor, but
    /// can be any function taking the capability's operation type and returning
    /// the effect type.
    ///
    /// This will likely only be called from the implementation of [`WithContext`]
    /// for the app's `Capabilities` type.
    pub fn with_effect<OtherT, F>(&self, func: F) -> CapabilityContext<OtherT, Ev>
    where
        F: Fn(OtherT) -> T + Sync + Send + Copy + 'static,
        OtherT: 'static,
    {
        let inner = Arc::new(ContextInner {
            steps: self.inner.steps.map_effect(func),
            events: self.inner.events.clone(),
            spawner: self.inner.spawner.clone(),
        });

        CapabilityContext { inner }
    }

    /// Transform the CapabilityContext into one maps each event passed to
    /// `update_app` using the provided function.
    ///
    /// This is useful when composing apps from modules to wrap a submodule's
    /// event type with a specific variant of the parent module's event, so it can
    /// be forwarded to the submodule when received.
    ///
    /// In a typical case you would implement `From` on the submodule's `Capabilities` type
    ///
    /// ```rust,ignore
    /// impl From<&ParentModuleCapabilities> for SubmoduleCapabilities {
    ///     fn from(incoming: &ParentModuleCapabilities) -> Self {
    ///         SubmoduleCapabilities {
    ///             some_capability: incoming.some_capability.map_event(Event::Submodule),
    ///             render: incoming.render.map_event(Event::Submodule),
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// in the parent module's `update` function, you can then call `.into()` on the
    /// capabilities, before passing them down to the submodule.
    pub fn map_event<NewEv, F>(&self, func: F) -> CapabilityContext<T, NewEv>
    where
        F: Fn(NewEv) -> Ev + Sync + Send + 'static,
        NewEv: 'static,
    {
        let inner = Arc::new(ContextInner {
            steps: self.inner.steps.clone(),
            events: self.inner.events.map_input(func),
            spawner: self.inner.spawner.clone(),
        });

        CapabilityContext { inner }
    }

    pub(crate) fn send_step(&self, step: Step<T>) {
        self.inner.steps.send(step);
    }
}

#[cfg(test)]
mod tests {
    use static_assertions::assert_impl_all;

    use super::*;

    #[allow(dead_code)]
    enum Effect {}

    #[allow(dead_code)]
    enum Event {}

    assert_impl_all!(CapabilityContext<Effect, Event>: Send, Sync);
}
