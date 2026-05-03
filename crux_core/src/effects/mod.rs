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

use std::sync::Arc;

use crate::Core;
use crate::Request;
use crate::Resolvable;
use crate::ResolveError;
use crate::capability::Operation;

pub use registry::Registry;

pub struct EffectRouter<App: crate::App> {
    core: Core<App>,
    route_effects: Box<dyn Fn(App::Effect) + Send + Sync>,
}

impl<App: crate::App> EffectRouter<App> {
    pub fn new<F, R>(core: Core<App>, route_effects_builder: F) -> Arc<Self>
    where
        F: FnOnce(std::sync::Weak<Self>) -> R,
        R: Fn(App::Effect) + Send + Sync + 'static,
    {
        Arc::new_cyclic(|weak| {
            let route_effects = route_effects_builder(weak.clone());
            Self {
                core,
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

    pub fn process(&self) {
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
    fn resolve_request(&self, request: Request<Op>, output: Op::Output)
    -> Result<(), ResolveError>;
}

impl<App: crate::App, Op> ResolveSink<Op> for EffectRouter<App>
where
    Op: Operation,
{
    fn resolve_request(
        &self,
        mut request: Request<Op>,
        output: Op::Output,
    ) -> Result<(), ResolveError> {
        self.resolve(&mut request, output)
    }
}
