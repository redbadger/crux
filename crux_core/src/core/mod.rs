mod effect;
mod observability;
mod request;
mod resolve;

use std::collections::VecDeque;
use std::sync::{Mutex, RwLock};

pub use effect::{Effect, EffectFFI};
pub use observability::{CoreRuntimeObserver, RuntimeContextId};
pub use request::Request;
pub use resolve::{EffectVariant, OperationKind, RequestHandle, Resolvable, ResolveError};

pub use observability::RuntimeObservationContext;

use crate::{App, Command};

/// The Crux core. Create an instance of this type with your App type as the type parameter
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
    app: A,

    // internals
    root_command: Mutex<Command<A::Effect, A::Event>>,
    observation_context: Option<RuntimeObservationContext>,
}
// ANCHOR_END: core

impl<A> Core<A>
where
    A: App + Default,
    A::Model: Default,
{
    /// Create an instance of the Crux core to start a Crux application, e.g.
    ///
    /// If you want to construct [App] and [`App::Model`] yourself instead of using default, call [`Self::new_with`]
    ///
    /// ```rust,ignore
    /// let core: Core<MyApp> = Core::new();
    /// ```
    ///
    #[must_use]
    pub fn new() -> Self {
        let root_command = Command::done();

        Self {
            model: RwLock::default(),
            app: A::default(),
            root_command: Mutex::new(root_command),
            observation_context: None,
        }
    }
}

impl<A> Core<A>
where
    A: App,
{
    /// Create an instance of the Crux core to start a Crux application with a given [App] and [`App::Model`]
    ///
    /// ```rust,ignore
    /// let app = MyApp::new();
    /// let model = MyModel::new();
    /// let core = Core::new_with(app, model);
    /// ```
    ///
    #[must_use]
    pub fn new_with(app: A, model: A::Model) -> Self {
        let root_command = Command::done();

        Self {
            model: RwLock::new(model),
            app,
            root_command: Mutex::new(root_command),
            observation_context: None,
        }
    }

    #[must_use]
    pub fn with_runtime_observer(
        mut self,
        observer: impl CoreRuntimeObserver + Send + Sync + 'static,
    ) -> Self {
        self.observation_context = Some(RuntimeObservationContext::new(observer));
        self
    }

    /// Run the app's `update` function with a given `event`, returning a vector of
    /// effect requests.
    ///
    /// # Panics
    ///
    /// Panics if the model `RwLock` was poisoned.
    // used in docs/internals/runtime.md
    // ANCHOR: process_event
    pub fn process_event(&self, event: A::Event) -> Vec<A::Effect> {
        let event_context_id = self.observe_event_start(Some(RuntimeContextId::ROOT));

        let mut model = self.model.write().expect("Model RwLock was poisoned.");
        let command = self.app.update(event, &mut model);
        drop(model);

        let mut root_command = self
            .root_command
            .lock()
            .expect("Capability runtime lock was poisoned");
        root_command.spawn(|ctx| self.host_event_command(command, event_context_id, ctx));

        drop(root_command);

        self.process()
    }
    // ANCHOR_END: process_event

    /// Resolve an effect `request` for operation `Op` with the corresponding result.
    ///
    /// Note that the `request` is borrowed mutably. When a request that is expected to
    /// only be resolved once is passed in, it will be consumed and changed to a request
    /// which can no longer be resolved.
    ///
    /// # Errors
    ///
    /// Errors if the request cannot (or should not) be resolved.
    // used in docs/internals/runtime.md and docs/internals/bridge.md
    // ANCHOR: resolve
    // ANCHOR: resolve_sig
    pub fn resolve<Output>(
        &self,
        request: &mut impl Resolvable<Output>,
        result: Output,
    ) -> Result<Vec<A::Effect>, ResolveError>
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
        let mut root_command = self
            .root_command
            .lock()
            .expect("Capability runtime lock was poisoned");

        let mut events: VecDeque<_> = root_command.events_with_context().collect();

        while let Some(command_event) = events.pop_front() {
            let event_owner = command_event.owner;
            let event_context_id = self.observe_event_start(command_event.parent_context_id);
            let mut model = self.model.write().expect("Model RwLock was poisoned.");
            let command = self.app.update(command_event.event, &mut model);
            drop(model);

            // Follow-up work belongs to the command that emitted the event.
            event_owner.spawn(|ctx| self.host_event_command(command, event_context_id, ctx));
            drop(event_owner);

            events.extend(root_command.events_with_context());
        }

        root_command.effects().collect()
    }
    // ANCHOR_END: process

    /// Allocate an event identity and report its start when observation is enabled.
    /// Unobserved events need neither an identity nor a parent context ID.
    fn observe_event_start(
        &self,
        parent_context_id: Option<RuntimeContextId>,
    ) -> Option<RuntimeContextId> {
        self.observation_context
            .as_ref()
            .map(|observation_context| {
                let context_id = observation_context
                    .shared_context_id_generator
                    .next_context_id();
                observation_context.observer.event_started(
                    context_id,
                    parent_context_id.expect("observed event must have a parent context ID"),
                    std::any::type_name::<A::Event>(),
                );
                context_id
            })
    }

    /// Add observation only when enabled. Both paths keep the caller's chosen
    /// owner, so structured concurrency does not depend on observability.
    fn host_event_command(
        &self,
        command: Command<A::Effect, A::Event>,
        event_context_id: Option<RuntimeContextId>,
        host_context: crate::command::CommandContext<A::Effect, A::Event>,
    ) -> impl std::future::Future<Output = ()> + Send + 'static + use<A> {
        match &self.observation_context {
            Some(observation_context) => futures::future::Either::Left(command.host_observably(
                host_context,
                event_context_id.expect("observed event must have an identity"),
                observation_context.clone(),
            )),
            None => futures::future::Either::Right(command.into_future(host_context)),
        }
    }

    /// Get the current state of the app's view model.
    ///
    /// # Panics
    ///
    /// Panics if the model lock was poisoned.
    pub fn view(&self) -> A::ViewModel {
        let model = self.model.read().expect("Model RwLock was poisoned.");

        self.app.view(&model)
    }
}

impl<A> Default for Core<A>
where
    A: App + Default,
    A::Model: Default,
{
    fn default() -> Self {
        Self::new()
    }
}
