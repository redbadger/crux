mod effect;
mod request;
mod resolve;

use std::sync::RwLock;

pub use effect::Effect;
pub use request::Request;
pub use resolve::ResolveError;

pub(crate) use resolve::Resolve;

use crate::capability::CommandSpawner;
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
// used in docs/internals/runtime.md
// ANCHOR: core
pub struct Core<A>
where
    A: App,
{
    // WARNING: The user controlled types _must_ be defined first
    // so that they are dropped first, in case they contain coordination
    // primitives which attempt to wake up a future when dropped. For that
    // reason the executor _must_ outlive the user type instances

    // user types
    model: RwLock<A::Model>,
    capabilities: A::Capabilities,
    app: A,

    // internals
    requests: Receiver<A::Effect>,
    capability_events: Receiver<A::Event>,
    executor: QueuingExecutor,

    // temporary command support
    command_spawner: CommandSpawner<A::Effect, A::Event>,
}
// ANCHOR_END: core

impl<A> Core<A>
where
    A: App,
{
    /// Create an instance of the Crux core to start a Crux application, e.g.
    ///
    /// ```rust,ignore
    /// let core: Core<MyApp> = Core::new();
    /// ```
    ///
    pub fn new() -> Self
    where
        A::Capabilities: WithContext<A::Event, A::Effect>,
    {
        let (request_sender, request_receiver) = capability::channel();
        let (event_sender, event_receiver) = capability::channel();
        let (executor, spawner) = capability::executor_and_spawner();
        let proto_context = ProtoContext::new(request_sender, event_sender, spawner);
        let command_spawner = CommandSpawner::new(proto_context.clone());

        Self {
            model: Default::default(),
            executor,
            app: Default::default(),
            capabilities: <<A as App>::Capabilities>::new_with_context(proto_context),
            requests: request_receiver,
            capability_events: event_receiver,
            command_spawner,
        }
    }

    /// Run the app's `update` function with a given `event`, returning a vector of
    /// effect requests.
    // used in docs/internals/runtime.md
    // ANCHOR: process_event
    pub fn process_event(&self, event: A::Event) -> Vec<A::Effect> {
        let mut model = self.model.write().expect("Model RwLock was poisoned.");

        let command = self.app.update(event, &mut model, &self.capabilities);

        // drop the model here, we don't want to hold the lock for the process() call
        drop(model);

        self.command_spawner.spawn(command);
        self.process()
    }
    // ANCHOR_END: process_event

    /// Resolve an effect `request` for operation `Op` with the corresponding result.
    ///
    /// Note that the `request` is borrowed mutably. When a request that is expected to
    /// only be resolved once is passed in, it will be consumed and changed to a request
    /// which can no longer be resolved.
    // used in docs/internals/runtime.md and docs/internals/bridge.md
    // ANCHOR: resolve
    // ANCHOR: resolve_sig
    pub fn resolve<Op>(
        &self,
        request: &mut Request<Op>,
        result: Op::Output,
    ) -> Result<Vec<A::Effect>, ResolveError>
    where
        Op: Operation,
        // ANCHOR_END: resolve_sig
    {
        let resolve_result = request.resolve(result);
        debug_assert!(resolve_result.is_ok());

        resolve_result?;

        Ok(self.process())
    }
    // ANCHOR_END: resolve

    // used in docs/internals/runtime.md
    // ANCHOR: process
    pub(crate) fn process(&self) -> Vec<A::Effect> {
        self.executor.run_all();

        while let Some(capability_event) = self.capability_events.receive() {
            let mut model = self.model.write().expect("Model RwLock was poisoned.");
            let command = self
                .app
                .update(capability_event, &mut model, &self.capabilities);

            drop(model);

            self.command_spawner.spawn(command);
            self.executor.run_all();
        }

        self.requests.drain().collect()
    }
    // ANCHOR_END: process

    /// Get the current state of the app's view model.
    pub fn view(&self) -> A::ViewModel {
        let model = self.model.read().expect("Model RwLock was poisoned.");

        self.app.view(&model)
    }
}

impl<A> Default for Core<A>
where
    A: App,
    A::Capabilities: WithContext<A::Event, A::Effect>,
{
    fn default() -> Self {
        Self::new()
    }
}
