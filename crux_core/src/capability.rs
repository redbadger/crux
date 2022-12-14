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
//! ```rust,ignore
//! use crux_http::{Http, HttpRequest};
//!
//! enum Effect {
//!     Http(HttpRequest)
//! }
//!
//! #[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
//! struct Capabilities {
//!     http: Http<Effect>
//! }
//!
//! impl crux_core::WithContext<App, Effect> for Capabilities {
//!    fn new_with_context(context: CapabilityContext<Effect, Event>) -> Capabilities {
//!        Capabilities {
//!            http: Http::new(context.with_effect(Effect::Http)),
//!        }
//!    }
//! }
//! ```
//!
//! This links the (reusable) capabilities with the app-specific `Effect` type, used by the Shell to dispatch
//! side-effect requests to the right capability implementation (and, in some languages, checking that all necessary
//! capabilities are implemented).
//!
//! # Implementing a capability
//!
//! TODO. See the crux_http crate as a working example.

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
    /// can be used to interact with the Shell.
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
    /// This is useful when composing apps from modules to wrap submodule's
    /// event type with a specific variant of parent module event, so it can
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
