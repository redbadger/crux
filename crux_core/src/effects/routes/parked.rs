use std::sync::{Arc, Weak};

use crate::{
    Request, ResolveError,
    capability::Operation,
    effects::{EffectId, EffectRouter, Routes, registry::Registry},
};

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
