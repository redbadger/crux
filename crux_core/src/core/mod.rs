mod effect;
mod request;
mod resolve;

use std::sync::RwLock;

pub use effect::Effect;
pub use request::Request;
pub use resolve::ResolveError;

pub(crate) use resolve::Resolve;

use crate::capability::{self, channel::Receiver, Operation, ProtoContext, QueuingExecutor};
use crate::{App, WithContext};

/// The Crux core. Create an instance of this type with your effect type, and your app type as type parameters
///
/// The core interface allows passing in events of type `A::Event` using [`Core::process_event`].
/// It will return back an effect of type `Ef`, containing an effect request, with the input needed for processing
/// the effect. the `Effect` type can be used by shells to dispatch to the right capability implementation.
///
/// The result of the capability's work can then be sent back to the core using [`Core::resolve`], passing
/// in the request and the corresponding capability output type.
pub struct Core<Ef, A>
where
    A: App,
{
    model: RwLock<A::Model>,
    executor: QueuingExecutor,
    capabilities: A::Capabilities,
    requests: Receiver<Ef>,
    capability_events: Receiver<A::Event>,
    app: A,
}

impl<Ef, A> Core<Ef, A>
where
    Ef: Effect,
    A: App,
{
    /// Create an instance of the Crux core to start a Crux application, e.g.
    ///
    /// ```rust,ignore
    /// let core: Core<HelloEffect, Hello> = Core::new::<HelloCapabilities>();
    /// ```
    ///
    pub fn new<Capabilities>() -> Self
    where
        Capabilities: WithContext<A, Ef>,
    {
        let (request_sender, request_receiver) = capability::channel();
        let (event_sender, event_receiver) = capability::channel();
        let (executor, spawner) = capability::executor_and_spawner();
        let capability_context = ProtoContext::new(request_sender, event_sender, spawner);

        Self {
            model: Default::default(),
            executor,
            app: Default::default(),
            capabilities: Capabilities::new_with_context(capability_context),
            requests: request_receiver,
            capability_events: event_receiver,
        }
    }

    /// Run the app's `update` function with a given `event`, returning a vector of
    /// effect requests.
    pub fn process_event(&self, event: A::Event) -> Vec<Ef> {
        let mut model = self.model.write().expect("Model RwLock was poisoned.");

        self.app.update(event, &mut model, &self.capabilities);

        self.process()
    }

    /// Resolve an effect `request` for operation `Op` with the corresponding result.
    ///
    /// Not that the `request` is borrowed mutably. When a request expected to only be
    /// resolved once is passed in, it will be consumed and changed to a request which can
    /// no longer be resolved.
    pub fn resolve<Op>(&self, request: &mut Request<Op>, result: Op::Output) -> Vec<Ef>
    where
        Op: Operation,
    {
        let resolve_result = request.resolve(result);
        debug_assert!(resolve_result.is_ok());

        self.process()
    }

    pub(crate) fn process(&self) -> Vec<Ef> {
        self.executor.run_all();

        while let Some(capability_event) = self.capability_events.receive() {
            let mut model = self.model.write().expect("Model RwLock was poisoned.");
            self.app
                .update(capability_event, &mut model, &self.capabilities);
            drop(model);
            self.executor.run_all();
        }

        self.requests.drain().collect()
    }

    /// Get the current state of the app's view model.
    pub fn view(&self) -> A::ViewModel {
        let model = self.model.read().expect("Model RwLock was poisoned.");

        self.app.view(&model)
    }
}

impl<Ef, A> Default for Core<Ef, A>
where
    Ef: Effect,
    A: App,
    A::Capabilities: WithContext<A, Ef>,
{
    fn default() -> Self {
        Self::new::<A::Capabilities>()
    }
}
