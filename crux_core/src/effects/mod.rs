//! Support for effect handling inside the core
//!
//! This module enables and advanced use-case, where some effects are not
//! handled by the shell using the standard serialization-based FFI interface.
//! Instead the core FFI can be extended with core-side effect processing or
//! custom effect handling FFIs APIs handing the operations and outputs using
//! a different data exchange method (e.g. raw pointers, zero-copy format like
//! cap'n proto, etc.)
//!
//! TODO complete docs

mod registry;
pub mod routes;

use std::sync::{Arc, Weak};

use crate::{Core, Request, Resolvable, ResolveError, capability::Operation};

pub struct EffectRouter<App, RouteSet>
where
    App: crate::App,
{
    pub routes: RouteSet,
    core: Core<App>,
    route_effects: Box<dyn Fn(App::Effect) + Send + Sync>,
}

pub trait Routes<App>: Sized + Clone
where
    App: crate::App,
{
    fn new(router: Weak<EffectRouter<App, Self>>) -> Self;
}

impl<App, RouteSet> EffectRouter<App, RouteSet>
where
    App: crate::App,
    RouteSet: Routes<App> + Send + Sync + 'static,
{
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

    pub fn view(&self) -> App::ViewModel {
        self.core.view()
    }

    fn process(&self) {
        for effect in self.core.process() {
            (self.route_effects)(effect);
        }
    }
}

pub trait ResolveSink<Op>
where
    Op: Operation,
{
    /// Resolve a [`Request`] with an `output` value.
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
