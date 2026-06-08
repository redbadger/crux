use std::sync::{Arc, Weak};

use crate::{
    Request, ResolveError,
    capability::Operation,
    effects::{EffectId, EffectRouter, Routes, registry::Registry},
};

/// A route for effects handled over a custom, user-owned FFI.
///
/// The "opaque typed" lane is for operations whose payloads or results are
/// awkward or undesirable to serialize, such as pointer-style handles or large
/// non-serializable buffers. Instead of bytes, the request is *parked* under an
/// [`EffectId`] with [`Parked::park`], and the shell receives that id together
/// with the (typed) operation. When the shell has a result, it calls
/// [`Parked::resolve`] with the id and the typed output.
///
/// `Parked` keeps a [`Weak`] reference to its [`EffectRouter`] so that resolving
/// a request can advance the runtime and route any follow-up effects.
pub struct Parked<A, RouteSet, Op>
where
    A: crate::App,
    RouteSet: Routes<A>,
    Op: Operation,
{
    router: Weak<EffectRouter<A, RouteSet>>,
    registry: Registry<Op>,
}

impl<App, RouteSet, Op> Parked<App, RouteSet, Op>
where
    App: crate::App,
    RouteSet: Routes<App> + Send + Sync + 'static,
    Op: Operation,
{
    /// Create a parked route attached to `router`.
    ///
    /// Called from your [`Routes::new`] implementation with the [`Weak`] router
    /// handle the trait provides.
    #[must_use]
    pub fn new(router: Weak<EffectRouter<App, RouteSet>>) -> Self {
        Self {
            router,
            registry: Registry::default(),
        }
    }

    /// Resume a parked request and route any follow-up effects.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying request could not be resolved.
    ///
    /// # Panics
    ///
    /// Panics if the router has been dropped, or the internal registry lock has
    /// been poisoned.
    pub fn resolve(
        &self,
        id: EffectId<Op::Output>,
        output: Op::Output,
    ) -> Result<(), ResolveError> {
        self.registry.resolve(id, output)?;
        self.router().process();

        Ok(())
    }

    fn router(&self) -> Arc<EffectRouter<App, RouteSet>> {
        self.router.upgrade().expect("effect router dropped")
    }
}

impl<App, RouteSet, Op> Parked<App, RouteSet, Op>
where
    App: crate::App,
    RouteSet: Routes<App>,
    Op: Operation,
{
    /// Park a request under an ID for a custom FFI to resume later.
    ///
    /// # Panics
    ///
    /// Panics if the internal registry lock has been poisoned.
    #[must_use]
    pub fn park(&self, request: Request<Op>) -> (EffectId<Op::Output>, Op) {
        let (id, operation) = self.registry.register(request);

        (id, operation)
    }
}
