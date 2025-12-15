mod effect;
mod request;
mod resolve;

use std::collections::VecDeque;
use std::sync::{Mutex, RwLock};

pub use effect::{Effect, EffectFFI};
pub use request::Request;
pub use resolve::{RequestHandle, Resolvable, ResolveError};

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
    root_command: Mutex<Command<A::Effect, A::Event, A::Model>>,
}
// ANCHOR_END: core

impl<A> Core<A>
where
    A: App + Send,
{
    /// Create an instance of the Crux core to start a Crux application, e.g.
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
        }
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
        let command = Command::new_with_model(|ctx| async move {
            let command = ctx.model(|model| self.app.update(event, model)).await;
            command.into_future(ctx).await;
        });

        let mut root_command = self
            .root_command
            .lock()
            .expect("Capability runtime lock was poisoned");
        root_command.spawn(|ctx| command.into_future(ctx));

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
        let mut model = self.model.write().expect("Model RwLock was poisoned.");
        root_command.run_until_settled(Some(&mut *model));
        // drop the model here, we don't want to hold the lock for the process() call
        drop(model);

        let mut events: VecDeque<_> = root_command.events().collect();

        while let Some(event_from_commands) = events.pop_front() {
            let mut model = self.model.write().expect("Model RwLock was poisoned.");
            let command = self.app.update(event_from_commands, &mut model);
            drop(model);

            root_command.spawn(|ctx| command.into_future(ctx));

            events.extend(root_command.events());
        }

        root_command.effects().collect()
    }
    // ANCHOR_END: process

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
    A: App,
{
    fn default() -> Self {
        Self::new()
    }
}
