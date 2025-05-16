//! Testing support for unit testing Crux apps.
use anyhow::Result;
use std::{collections::VecDeque, sync::Arc};

use crate::{
    capability::{
        channel::Receiver, executor_and_spawner, CommandSpawner, Operation, ProtoContext,
        QueuingExecutor,
    },
    Command, Request, WithContext,
};

/// AppTester is a simplified execution environment for Crux apps for use in
/// tests.
///
/// Please note that the AppTester is strictly no longer required now that Crux
/// has a new [`Command`] API. To test apps without the AppTester, you can call
/// the `update` method on your app directly, and then inspect the effects
/// returned by the command. For examples of how to do this, consult any of the
/// [examples in the Crux repository](https://github.com/redbadger/crux/tree/master/examples).
/// The AppTester is still provided for backwards compatibility, and to allow you to
/// migrate to the new API without changing the tests,
/// giving you increased confidence in your refactor.
///
/// Create an instance of `AppTester` with your `App` and an `Effect` type
/// using [`AppTester::default`].
///
/// for example:
///
/// ```rust,ignore
/// let app = AppTester::<ExampleApp, ExampleEffect>::default();
/// ```
pub struct AppTester<App>
where
    App: crate::App,
{
    app: App,
    capabilities: App::Capabilities,
    context: Arc<AppContext<App::Effect, App::Event>>,
    command_spawner: CommandSpawner<App::Effect, App::Event>,
}

struct AppContext<Ef, Ev> {
    commands: Receiver<Ef>,
    events: Receiver<Ev>,
    executor: QueuingExecutor,
}

impl<App> AppTester<App>
where
    App: crate::App,
{
    /// Create an `AppTester` instance for an existing app instance. This can be used if your App
    /// has a constructor other than `Default`, for example when used as a child app and expecting
    /// configuration from the parent
    pub fn new(app: App) -> Self
    where
        App::Capabilities: WithContext<App::Event, App::Effect>,
    {
        Self {
            app,
            ..Default::default()
        }
    }

    /// Run the app's `update` function with an event and a model state
    ///
    /// You can use the resulting [`Update`] to inspect the effects which were requested
    /// and potential further events dispatched by capabilities.
    pub fn update(
        &self,
        event: App::Event,
        model: &mut App::Model,
    ) -> Update<App::Effect, App::Event> {
        let command = self.app.update(event, model, &self.capabilities);
        self.command_spawner.spawn(command);

        self.context.updates()
    }

    /// Resolve an effect `request` from previous update with an operation output.
    ///
    /// This potentially runs the app's `update` function if the effect is completed, and
    /// produce another `Update`.
    pub fn resolve<Op: Operation>(
        &self,
        request: &mut Request<Op>,
        value: Op::Output,
    ) -> Result<Update<App::Effect, App::Event>> {
        request.resolve(value)?;

        Ok(self.context.updates())
    }

    /// Resolve an effect `request` from previous update, then run the resulting event
    ///
    /// This helper is useful for the common case where  one expects the effect to resolve
    /// to exactly one event, which should then be run by the app.
    #[track_caller]
    pub fn resolve_to_event_then_update<Op: Operation>(
        &self,
        request: &mut Request<Op>,
        value: Op::Output,
        model: &mut App::Model,
    ) -> Update<App::Effect, App::Event> {
        request.resolve(value).expect("failed to resolve request");
        let event = self.context.updates().expect_one_event();
        self.update(event, model)
    }

    /// Run the app's `view` function with a model state
    pub fn view(&self, model: &App::Model) -> App::ViewModel {
        self.app.view(model)
    }
}

impl<App> Default for AppTester<App>
where
    App: crate::App,
    App::Capabilities: WithContext<App::Event, App::Effect>,
{
    fn default() -> Self {
        let (command_sender, commands) = crate::capability::channel();
        let (event_sender, events) = crate::capability::channel();
        let (executor, spawner) = executor_and_spawner();
        let capability_context = ProtoContext::new(command_sender, event_sender, spawner);
        let command_spawner = CommandSpawner::new(capability_context.clone());

        Self {
            app: App::default(),
            capabilities: App::Capabilities::new_with_context(capability_context),
            context: Arc::new(AppContext {
                commands,
                events,
                executor,
            }),
            command_spawner,
        }
    }
}

impl<App> AsRef<App::Capabilities> for AppTester<App>
where
    App: crate::App,
{
    fn as_ref(&self) -> &App::Capabilities {
        &self.capabilities
    }
}

impl<Ef, Ev> AppContext<Ef, Ev> {
    pub fn updates(self: &Arc<Self>) -> Update<Ef, Ev> {
        self.executor.run_all();
        let effects = self.commands.drain().collect();
        let events = self.events.drain().collect();

        Update { effects, events }
    }
}

/// Update test helper holds the result of running an app update using [`AppTester::update`]
/// or resolving a request with [`AppTester::resolve`].
#[derive(Debug)]
#[must_use]
pub struct Update<Ef, Ev> {
    /// Effects requested from the update run
    pub effects: Vec<Ef>,
    /// Events dispatched from the update run
    pub events: Vec<Ev>,
}

impl<Ef, Ev> Update<Ef, Ev> {
    pub fn into_effects(self) -> impl Iterator<Item = Ef> {
        self.effects.into_iter()
    }

    pub fn effects(&self) -> impl Iterator<Item = &Ef> {
        self.effects.iter()
    }

    pub fn effects_mut(&mut self) -> impl Iterator<Item = &mut Ef> {
        self.effects.iter_mut()
    }

    /// Assert that the update contains exactly one effect and zero events,
    /// and return the effect
    #[track_caller]
    pub fn expect_one_effect(mut self) -> Ef {
        if self.events.is_empty() && self.effects.len() == 1 {
            self.effects.pop().unwrap()
        } else {
            panic!(
                "Expected one effect but found {} effect(s) and {} event(s)",
                self.effects.len(),
                self.events.len()
            );
        }
    }

    /// Assert that the update contains exactly one event and zero effects,
    /// and return the event
    #[track_caller]
    pub fn expect_one_event(mut self) -> Ev {
        if self.effects.is_empty() && self.events.len() == 1 {
            self.events.pop().unwrap()
        } else {
            panic!(
                "Expected one event but found {} effect(s) and {} event(s)",
                self.effects.len(),
                self.events.len()
            );
        }
    }

    /// Assert that the update contains no effects or events
    #[track_caller]
    pub fn assert_empty(self) {
        if self.effects.is_empty() && self.events.is_empty() {
            return;
        }
        panic!(
            "Expected empty update but found {} effect(s) and {} event(s)",
            self.effects.len(),
            self.events.len()
        );
    }

    /// Take effects matching the `predicate` out of the [`Update`]
    /// and return them, mutating the `Update`
    pub fn take_effects<P>(&mut self, predicate: P) -> VecDeque<Ef>
    where
        P: FnMut(&Ef) -> bool,
    {
        let (matching_effects, other_effects) = self.take_effects_partitioned_by(predicate);

        self.effects = other_effects.into_iter().collect();

        matching_effects
    }

    /// Take all of the effects out of the [`Update`]
    /// and split them into those matching `predicate` and the rest
    pub fn take_effects_partitioned_by<P>(&mut self, predicate: P) -> (VecDeque<Ef>, VecDeque<Ef>)
    where
        P: FnMut(&Ef) -> bool,
    {
        std::mem::take(&mut self.effects)
            .into_iter()
            .partition(predicate)
    }
}

impl<Effect, Event> Command<Effect, Event>
where
    Effect: Send + 'static,
    Event: Send + 'static,
{
    /// Assert that the Command contains _exactly_ one effect and zero events,
    /// and return the effect
    #[track_caller]
    pub fn expect_one_effect(&mut self) -> Effect {
        if self.events().next().is_some() {
            panic!("expected only one effect, but found an event");
        }
        let mut effects = self.effects();
        match (effects.next(), effects.next()) {
            (None, _) => panic!("expected one effect but got none"),
            (Some(effect), None) => effect,
            _ => panic!("expected one effect but got more than one"),
        }
    }
}

/// Panics if the pattern doesn't match an `Effect` from the specified `Update`
///
/// Like in a `match` expression, the pattern can be optionally followed by `if`
/// and a guard expression that has access to names bound by the pattern.
///
/// # Example
///
/// ```
/// # use crux_core::testing::Update;
/// # enum Effect { Render(String) };
/// # enum Event { None };
/// # let effects = vec![Effect::Render("test".to_string())].into_iter().collect();
/// # let mut update = Update { effects, events: vec!(Event::None) };
/// use crux_core::assert_effect;
/// assert_effect!(update, Effect::Render(_));
/// ```
#[macro_export]
macro_rules! assert_effect {
    ($expression:expr, $(|)? $( $pattern:pat_param )|+ $( if $guard: expr )? $(,)?) => {
        assert!($expression.effects().any(|e| matches!(e, $( $pattern )|+ $( if $guard )?)));
    };
}
