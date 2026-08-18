mod effect;
mod event;
mod observability;
mod request;
mod resolve;

use std::collections::VecDeque;
use std::sync::{Mutex, RwLock};

pub use effect::{Effect, EffectFFI};
pub use event::EventName;
pub use observability::{
    CommandObservation, CoreRuntimeObserver, RuntimeContextId, RuntimeObservation,
};
pub use request::Request;
pub use resolve::{EffectVariant, OperationKind, RequestHandle, Resolvable, ResolveError};

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
    observation_context: Option<RuntimeObservation<A::Event>>,
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
        Self {
            model: RwLock::default(),
            app: A::default(),
            root_command: Mutex::new(Command::done()),
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
        Self {
            model: RwLock::new(model),
            app,
            root_command: Mutex::new(Command::done()),
            observation_context: None,
        }
    }

    /// Observe update/command lifetimes, effects, and causal relationships.
    ///
    /// Event names are obtained through [`EventName`], which can be derived
    /// without inspecting event payloads. Unobserved apps need no naming trait.
    #[must_use]
    pub fn with_runtime_observer(
        mut self,
        observer: impl CoreRuntimeObserver + Send + Sync + 'static,
    ) -> Self
    where
        A::Event: EventName,
    {
        self.observation_context = Some(RuntimeObservation::new(observer));
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
        let command_observation = self.observe_command_start(&event, None);

        let mut model = self.model.write().expect("Model RwLock was poisoned.");
        let command = self.app.update(event, &mut model);

        // drop the model here, we don't want to hold the lock for the process() call
        drop(model);

        let mut root_command = self
            .root_command
            .lock()
            .expect("Capability runtime lock was poisoned");
        match command_observation {
            Some(observation) => {
                root_command.spawn(|ctx| command.host_with_observation(ctx, observation));
            }
            None => root_command.spawn(|ctx| command.into_future(ctx)),
        }

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
            let command_observation =
                self.observe_command_start(&command_event.event, command_event.caused_by);

            let mut model = self.model.write().expect("Model RwLock was poisoned.");
            let command = self.app.update(command_event.event, &mut model);
            drop(model);

            // Run the returned command alongside the other active commands.
            if let Some(observation) = command_observation {
                root_command.spawn(|ctx| command.host_with_observation(ctx, observation));
            } else {
                root_command.spawn(|ctx| command.into_future(ctx));
            }

            events.extend(root_command.events_with_context());
        }

        root_command
            .effects_with_context()
            .filter_map(|effect| self.dispatch_effect(effect))
            .collect()
    }
    // ANCHOR_END: process

    /// Discard abandoned effects, then observe dispatch and release the payload.
    /// If the wait ended before guard delivery, discard the effect instead.
    fn dispatch_effect(
        &self,
        mut effect: crate::command::CommandEffect<A::Effect>,
    ) -> Option<A::Effect> {
        if !effect.is_pending() {
            return None;
        }

        let Some(observation) = &self.observation_context else {
            return Some(effect.into_effect());
        };

        let parent = effect.parent_context_id.unwrap_or(RuntimeContextId::ROOT);
        match effect.effect.kind() {
            OperationKind::Notify => {
                observation
                    .observer
                    .notification(parent, effect.effect.name());
            }
            kind @ (OperationKind::Request | OperationKind::Stream) => {
                let id = observation.next_context_id();
                observation
                    .observer
                    .effect_started(id, parent, effect.effect.name(), kind);
                let guard = crate::command::SpanGuard {
                    id,
                    observer: observation.observer.clone(),
                };
                if let Err(guard) = effect.set_guard(guard) {
                    drop(guard);
                    return None;
                }
            }
        }

        Some(effect.into_effect())
    }

    /// Start the combined update/command span before entering `App::update`.
    fn observe_command_start(
        &self,
        event: &A::Event,
        caused_by: Option<RuntimeContextId>,
    ) -> Option<CommandObservation> {
        self.observation_context
            .as_ref()
            .map(|context| context.start_command(event, caused_by))
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
