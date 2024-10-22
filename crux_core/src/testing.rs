//! Testing support for unit testing Crux apps.
use anyhow::Result;
use std::sync::Arc;

use crate::{
    capability::{channel::Receiver, Operation, ProtoContext, QueuingExecutor},
    Request, WithContext,
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
        let event = match command {
            crate::Command::None => None,
            crate::Command::Event(event) => Some(event),
            crate::Command::Effects(effects) => {
                for effect in effects {
                    self.context.executor.spawn_task(effect);
                }
                None
            }
        };
        self.context.updates(event)
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
        Ok(self.context.updates(None))
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

impl<Ef, Ev> AppContext<Ef, Ev> {
    pub fn updates(self: &Arc<Self>, pending_event: Option<Ev>) -> Update<Ef, Ev> {
        let mut events: Vec<Ev> = pending_event.into_iter().collect();
        loop {
            let commands = self.executor.run_all();
            if commands.is_empty() {
                break;
            }
            for c in commands {
                match c {
                    crate::Command::None => {}
                    crate::Command::Event(ev) => events.push(ev),
                    crate::Command::Effects(effs) => {
                        for task in effs {
                            self.executor.spawn_task(task);
                        }
                    }
                }
            }
        }
        let effects = self.commands.drain().collect();
        Update { effects, events }
    }
}

/// Update test helper holds the result of running an app update using [`AppTester::update`]
/// or resolving a request with [`AppTester::resolve`].
#[derive(Debug)]
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
