//! Testing support for unit testing Crux apps.
use anyhow::Result;
use std::{collections::VecDeque, sync::Arc};

use crate::{
    capability::{channel::Receiver, Operation, ProtoContext, QueuingExecutor},
    CommandInner, Request, WithContext,
};

/// AppTester is a simplified execution environment for Crux apps for use in
/// tests.
///
/// Create an instance of `AppTester` with your `App` and an `Effect` type
/// using [`AppTester::default`].
///
/// for example:
///
/// ```rust,ignore
/// let app = AppTester::<ExampleApp, ExampleEffect>::default();
/// ```
pub struct AppTester<App, Ef>
where
    App: crate::App,
{
    app: App,
    capabilities: App::Capabilities,
    context: Arc<AppContext<Ef, App::Event>>,
}

struct AppContext<Ef, Ev> {
    commands: Receiver<Ef>,
    executor: QueuingExecutor<Ev>,
}

impl<App, Ef> AppTester<App, Ef>
where
    App: crate::App,
{
    /// Create an `AppTester` instance for an existing app instance. This can be used if your App
    /// has a constructor other than `Default`, for example when used as a child app and expecting
    /// configuration from the parent
    pub fn new(app: App) -> Self
    where
        Ef: Send + 'static,
        App::Capabilities: WithContext<Ef>,
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
    pub fn update(&self, event: App::Event, model: &mut App::Model) -> Update<Ef, App::Event> {
        let command = self.app.update(event, model, &self.capabilities);
        let mut queue: VecDeque<_> = std::iter::once(command).collect();
        let mut events = Vec::new();
        while let Some(command) = queue.pop_front() {
            match command.inner {
                CommandInner::None => continue,
                CommandInner::Event(event) => {
                    events.push(event);
                }
                CommandInner::Effect(effect) => {
                    self.context.executor.spawn_task(effect);
                }
                CommandInner::Stream(stream) => {
                    self.context.executor.spawn_stream(stream);
                }
                CommandInner::Multiple(multiple) => {
                    for command in multiple {
                        queue.push_back(command);
                    }
                }
            };
        }
        self.context.updates(events)
    }

    /// Resolve an effect `request` from previous update with an operation output.
    ///
    /// This potentially runs the app's `update` function if the effect is completed, and
    /// produce another `Update`.
    pub fn resolve<Op: Operation>(
        &self,
        request: &mut Request<Op>,
        value: Op::Output,
    ) -> Result<Update<Ef, App::Event>> {
        request.resolve(value)?;
        Ok(self.context.updates(Vec::new()))
    }

    /// Resolve an effect `request` from previous update, then run the resulting event
    ///
    /// This helper is useful for the common case where  one expects the effect to resolve
    /// to exactly one event, which should then be run by the app.
    pub fn resolve_to_event_then_update<Op: Operation>(
        &self,
        request: &mut Request<Op>,
        value: Op::Output,
        model: &mut App::Model,
    ) -> Update<Ef, App::Event> {
        request.resolve(value).expect("failed to resolve request");
        let event = self.context.updates(Vec::new()).expect_one_event();
        self.update(event, model)
    }

    /// Run the app's `view` function with a model state
    pub fn view(&self, model: &App::Model) -> App::ViewModel {
        self.app.view(model)
    }
}

impl<App, Ef> Default for AppTester<App, Ef>
where
    App: crate::App,
    App::Capabilities: WithContext<Ef>,
    Ef: Send + 'static,
{
    fn default() -> Self {
        let (command_sender, commands) = crate::capability::channel();
        let executor = QueuingExecutor::new();
        let capability_context = ProtoContext::new(command_sender);

        Self {
            app: App::default(),
            capabilities: App::Capabilities::new_with_context(capability_context),
            context: Arc::new(AppContext { commands, executor }),
        }
    }
}

impl<App, Ef> AsRef<App::Capabilities> for AppTester<App, Ef>
where
    App: crate::App,
{
    fn as_ref(&self) -> &App::Capabilities {
        &self.capabilities
    }
}

impl<Ef, Ev> AppContext<Ef, Ev>
where
    Ev: 'static,
{
    pub fn updates(self: &Arc<Self>, mut events: Vec<Ev>) -> Update<Ef, Ev> {
        let mut queue: VecDeque<_> = self.executor.run_all().into_iter().collect();
        while let Some(command) = queue.pop_front() {
            match command.inner {
                CommandInner::None => {}
                CommandInner::Event(ev) => events.push(ev),
                CommandInner::Effect(effect) => {
                    self.executor.spawn_task(effect);
                    queue.extend(self.executor.run_all())
                }
                CommandInner::Stream(stream) => {
                    self.executor.spawn_stream(stream);
                    queue.extend(self.executor.run_all())
                }
                CommandInner::Multiple(commands) => queue.extend(commands),
            }
        }
        let effects = self.commands.drain().collect();
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
