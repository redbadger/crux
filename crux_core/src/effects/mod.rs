//! Support for routing effects to explicit, type-based handlers.
//!
//! This module enables an advanced use-case, where some effects are not
//! handled by the shell using the standard serialization-based FFI interface.
//! Instead the core FFI can be extended with core-side effect processing or
//! custom effect handling FFI APIs, handling the operations and outputs using
//! a different data exchange method (e.g. raw pointers, zero-copy formats like
//! Cap'n Proto, etc.)
//!
//! # Overview
//!
//! The entry point is the [`EffectRouter`], which wraps a [`Core`] and a
//! routing closure. The closure inspects each [`Effect`](crate::App::Effect)
//! the app emits and dispatches it to the appropriate handler, or "lane". Crucially,
//! the follow-up effects produced when a request is resolved are routed back
//! through the same closure, so the same policy applies for the whole lifetime
//! of a chain of effects.
//!
//! The available lanes live in the [`routes`] module:
//!
//! - [`Serialized`](routes::Serialized) keeps the standard, bridge-like
//!   behaviour: effects are serialized to bytes, sent to the shell, and
//!   resolved by id with serialized responses. This is the default lane and the
//!   primary onboarding path; it typically acts as the fall-through arm of the
//!   routing closure.
//! - [`Parked`](routes::Parked) supports payloads and results that are awkward
//!   or undesirable to serialize (for example opaque pointer-style handles),
//!   using a custom, user-owned FFI. The request is parked under an
//!   [`EffectId`] which the shell passes back when resolving.
//! - [`Buffer`](routes::Buffer) collects requests for the caller to drain and
//!   handle synchronously, which is useful in tests and simple in-process
//!   handlers.
//!
//! Effects can also be handled entirely inside the core by a Rust handler
//! (including async or background work) that resolves requests back through the
//! router via the [`ResolveSink`] trait.
//!
//! # Wiring it up
//!
//! Routes are grouped in a user-defined type that implements [`Routes`]. The
//! router is created with [`EffectRouter::new`], which hands the constructed
//! route set to a builder closure so it can be captured by the routing closure.
//! Because routes need to resolve effects back through the router, they hold a
//! [`Weak`] reference to it, and the router is therefore stored behind an
//! [`Arc`].
//!
//! See the `effect_router_prototype` integration test in `crux_core` for a
//! complete, worked example, and `docs/src/rfcs/effect-router.md` for the
//! design rationale.

mod registry;
pub mod routes;

use std::sync::{Arc, Weak};

use crate::{Core, Request, Resolvable, ResolveError, capability::Operation};

pub use registry::EffectId;

/// Wraps a [`Core`] and routes each emitted effect to a type-specific handler.
///
/// The router owns the set of routes (`RouteSet`) and a routing closure which
/// decides, per effect, which handler should process it. Any follow-up effects
/// produced while resolving a request are passed back through the same closure,
/// so routing decisions stay consistent across an entire chain of effects.
///
/// Construct one with [`EffectRouter::new`]. The router is always held behind an
/// [`Arc`] so that individual routes can keep a [`Weak`] reference back to it
/// and drive the runtime forward when they resolve requests.
pub struct EffectRouter<App, RouteSet>
where
    App: crate::App,
{
    /// The set of handlers effects are routed to.
    ///
    /// Exposed so the surrounding FFI type can reach individual routes (for
    /// example to resolve a parked or serialized request by id).
    pub routes: RouteSet,
    core: Core<App>,
    route_effects: Box<dyn Fn(App::Effect) + Send + Sync>,
}

/// A set of effect handlers ("routes") owned by an [`EffectRouter`].
///
/// Implement this on a type that groups together the individual routes your app
/// needs (for example a [`Serialized`](routes::Serialized) lane plus one or more
/// [`Parked`](routes::Parked) lanes and core-local handlers). The router calls
/// [`Routes::new`] while it is being constructed, handing over a [`Weak`]
/// reference to itself so each route can later resolve requests and advance the
/// runtime.
///
/// The type must be [`Clone`] because a clone is given to the routing closure
/// built in [`EffectRouter::new`]; routes are typically wrapped in [`Arc`] so
/// cloning is cheap and shares the same underlying handlers.
pub trait Routes<App>: Sized + Clone
where
    App: crate::App,
{
    /// Construct the route set, given a [`Weak`] handle to the router that will
    /// own it.
    ///
    /// The handle is `Weak` to avoid a reference cycle with the [`Arc`] holding
    /// the router; routes upgrade it on demand when they need to resolve a
    /// request.
    fn new(router: Weak<EffectRouter<App, Self>>) -> Self;
}

impl<App, RouteSet> EffectRouter<App, RouteSet>
where
    App: crate::App,
    RouteSet: Routes<App> + Send + Sync + 'static,
{
    /// Create a new router wrapping `core`.
    ///
    /// The route set is constructed first (via [`Routes::new`]) and then passed
    /// to `route_effects_builder`, which returns the routing closure. Splitting
    /// construction this way lets the closure capture the routes it needs to
    /// dispatch to, while the routes themselves hold a [`Weak`] reference back
    /// to the router so they can resolve requests later.
    ///
    /// The router is returned inside an [`Arc`] because that shared ownership is
    /// what the routes' weak references point at; see [`Arc::new_cyclic`].
    pub fn new<F, R>(core: Core<App>, route_effects_builder: F) -> Arc<Self>
    where
        F: FnOnce(RouteSet) -> R,
        R: Fn(App::Effect) + Send + Sync + 'static,
    {
        Arc::new_cyclic(|weak| {
            let routes = RouteSet::new(weak.clone());
            let route_effects = route_effects_builder(routes.clone());

            Self {
                core,
                routes,
                route_effects: Box::new(route_effects),
            }
        })
    }

    /// Process an event from the shell and route every resulting effect.
    ///
    /// This is the router's equivalent of [`Core::process_event`]: it forwards
    /// the event to the wrapped core and passes each emitted effect through the
    /// routing closure.
    pub fn update(&self, event: App::Event) {
        for effect in self.core.process_event(event) {
            (self.route_effects)(effect);
        }
    }

    /// Resolve an effect [`Request`] with an `output` value.
    ///
    /// # Errors
    ///
    /// Returns an error if the request is not expected to be resolved by
    /// the underlying [`Core`].
    pub fn resolve<Output>(
        &self,
        request: &mut impl Resolvable<Output>,
        output: Output,
    ) -> Result<(), ResolveError> {
        for effect in self.core.resolve(request, output)? {
            (self.route_effects)(effect);
        }

        Ok(())
    }

    /// Return the current view model from the wrapped [`Core`].
    pub fn view(&self) -> App::ViewModel {
        self.core.view()
    }

    /// Advance the core's effect runtime and route any follow-up effects.
    ///
    /// Routes call this after resolving a request whose `Output` type has been
    /// erased (so they cannot call [`EffectRouter::resolve`] directly): the
    /// request is resolved against the route's own registry, and this drives the
    /// runtime forward to collect and route the resulting effects.
    fn process(&self) {
        for effect in self.core.process() {
            (self.route_effects)(effect);
        }
    }
}

/// Lets a core-local handler resolve a [`Request`] back through the router.
///
/// Core-side handlers (for example background workers that do async I/O) hold a
/// [`Weak`] reference to something implementing this trait, typically the
/// [`EffectRouter`] itself. When a handler finishes its work, it calls
/// [`ResolveSink::resolve_request`] to resolve the original request and advance
/// the runtime, so any follow-up effects are routed using the same policy.
///
/// The trait is generic over the [`Operation`] `Op` so a handler only depends on
/// the single operation type it knows how to service, rather than the whole
/// route set.
pub trait ResolveSink<Op>
where
    Op: Operation,
{
    /// Resolve a [`Request`] with an `output` value and advance the runtime.
    ///
    /// # Errors
    ///
    /// Returns an error if the request is not expected to be resolved by
    /// the underlying [`Core`].
    fn resolve_request(
        &self,
        request: &mut Request<Op>,
        output: Op::Output,
    ) -> Result<(), ResolveError>;
}

impl<App, RouteSet, Op> ResolveSink<Op> for EffectRouter<App, RouteSet>
where
    App: crate::App,
    RouteSet: Routes<App> + Send + Sync + 'static,
    Op: Operation,
{
    fn resolve_request(
        &self,
        request: &mut Request<Op>,
        output: Op::Output,
    ) -> Result<(), ResolveError> {
        self.resolve(request, output)?;
        self.process();

        Ok(())
    }
}
