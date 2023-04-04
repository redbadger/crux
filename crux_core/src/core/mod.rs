mod effect;
mod request;
mod resolve;

use std::sync::RwLock;

pub use effect::Effect;
pub use request::Request;
pub use resolve::{Resolve, ResolveError};

use crate::{
    capability::{self, channel::Receiver, Operation, ProtoContext, QueuingExecutor},
    App, WithContext,
};

/// The Crux core. Create an instance of this type with your effect type, and your app type as type parameters
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
    /// lazy_static! {
    ///     static ref CORE: Core<HelloEffect, Hello> = Core::new::<HelloCapabilities>();
    /// }
    /// ```
    ///
    /// The core interface passes across messages serialized as bytes. These can be
    /// deserialized in the Shell using the types generated using the [typegen] module.
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

    pub fn process_event(&self, event: <A as App>::Event) -> Vec<Ef> {
        let mut model = self.model.write().expect("Model RwLock was poisoned.");

        self.app.update(event, &mut model, &self.capabilities);

        self.process()
    }

    pub fn resolve<Op>(&self, request: &mut Request<Op>, result: Op::Output) -> Vec<Ef>
    where
        Op: Operation,
    {
        let _ = request.resolve(result);

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

    /// Get the current state of the app's view model (serialized).
    pub fn view(&self) -> <A as App>::ViewModel {
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
