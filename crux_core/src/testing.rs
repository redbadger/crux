//! Testing support for unit testing Crux apps.
use std::rc::Rc;

use anyhow::Result;

use crate::{
    capability::{
        channel::Receiver, executor_and_spawner, Operation, ProtoContext, QueuingExecutor,
    },
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
    context: Rc<AppContext<Ef, App::Event>>,
}

struct AppContext<Ef, Ev> {
    commands: Receiver<Ef>,
    events: Receiver<Ev>,
    executor: QueuingExecutor,
}

impl<App, Ef> AppTester<App, Ef>
where
    App: crate::App,
{
    /// Run the app's `update` function with an event and a model state
    ///
    /// You can use the resulting [`Update`] to inspect the effects which were requested
    /// and potential further events dispatched by capabilities.
    pub fn update(&self, event: App::Event, model: &mut App::Model) -> Update<Ef, App::Event> {
        self.app.update(event, model, &self.capabilities);
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
    ) -> Result<Update<Ef, App::Event>> {
        request.resolve(value)?;

        Ok(self.context.updates())
    }

    /// Run the app's `view` function with a model state
    pub fn view(&self, model: &App::Model) -> App::ViewModel {
        self.app.view(model)
    }
}

impl<App, Ef> Default for AppTester<App, Ef>
where
    App: crate::App,
    App::Capabilities: WithContext<App, Ef>,
    App::Event: Send,
    Ef: Send + 'static,
{
    fn default() -> Self {
        let (command_sender, commands) = crate::capability::channel();
        let (event_sender, events) = crate::capability::channel();
        let (executor, spawner) = executor_and_spawner();
        let capability_context = ProtoContext::new(command_sender, event_sender, spawner);

        Self {
            app: App::default(),
            capabilities: App::Capabilities::new_with_context(capability_context),
            context: Rc::new(AppContext {
                commands,
                events,
                executor,
            }),
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
    pub fn updates(self: &Rc<Self>) -> Update<Ef, Ev> {
        self.executor.run_all();
        let effects = self.commands.drain().collect();
        let events = self.events.drain().collect();

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

/// Panics if the pattern doesn't match an `Effect` from the specified `Update`
///
/// Like in a `match` expression, the pattern can be optionally followed by `if`
/// and a guard expression that has access to names bound by the pattern.
///
/// # Example
///
/// ```
/// use crux_core::assert_effect;
/// # enum Effect { Render(String) };
/// # enum Event { None };
/// # let update = crux_core::testing::Update { effects: vec!(Effect::Render("test".to_string())), events: vec!(Event::None) };
/// assert_effect!(update, Effect::Render(_));
/// ```
#[macro_export]
macro_rules! assert_effect {
    ($expression:expr, $(|)? $( $pattern:pat_param )|+ $( if $guard: expr )? $(,)?) => {
        assert!($expression.effects.iter().any(|e| matches!(e, $( $pattern )|+ $( if $guard )?)));
    };
}
