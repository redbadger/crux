//! Testing support for unit testing Crux apps.
use std::{fmt, rc::Rc};

use crate::{
    capability::CapabilityContext,
    channels::Receiver,
    executor::{executor_and_spawner, QueuingExecutor},
    steps::StepRegistry,
    Request, Step, WithContext,
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
    commands: Receiver<Step<Ef>>,
    events: Receiver<Ev>,
    executor: QueuingExecutor,
    steps: StepRegistry,
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

    /// Run the app's `view` function with a model state
    pub fn view(&self, model: &mut App::Model) -> App::ViewModel {
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
        let (command_sender, commands) = crate::channels::channel();
        let (event_sender, events) = crate::channels::channel();
        let (executor, spawner) = executor_and_spawner();
        let capability_context = CapabilityContext::new(command_sender, event_sender, spawner);

        Self {
            app: App::default(),
            capabilities: App::Capabilities::new_with_context(capability_context),
            context: Rc::new(AppContext {
                commands,
                events,
                executor,
                steps: StepRegistry::default(),
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
        let effects = self
            .commands
            .drain()
            .map(|cmd| {
                let request = self.steps.register(cmd);
                TestEffect {
                    request,
                    context: Rc::clone(self),
                }
            })
            .collect();

        let events = self.events.drain().collect();

        Update { effects, events }
    }
}

/// Update test helper holds the result of running an app update using [`AppTester::update`].
#[derive(Debug)]
pub struct Update<Ef, Ev> {
    /// Effects requested from the update run
    pub effects: Vec<TestEffect<Ef, Ev>>,
    /// Events dispatched from the update run
    pub events: Vec<Ev>,
}

pub struct TestEffect<Ef, Ev> {
    request: Request<Ef>,
    context: Rc<AppContext<Ef, Ev>>,
}

impl<Ef, Ev> TestEffect<Ef, Ev> {
    pub fn resolve<T>(&self, result: &T) -> Update<Ef, Ev>
    where
        T: serde::ser::Serialize,
    {
        self.context.steps.resume(
            self.request.uuid.as_slice(),
            &bcs::to_bytes(result).unwrap(),
        );
        self.context.updates()
    }
}

impl<Ef, Ev> AsRef<Ef> for TestEffect<Ef, Ev> {
    fn as_ref(&self) -> &Ef {
        &self.request.effect
    }
}

impl<Ef, Ev> PartialEq<Ef> for TestEffect<Ef, Ev>
where
    Ef: PartialEq,
{
    fn eq(&self, other: &Ef) -> bool {
        self.request.effect == *other
    }
}

impl<Ef, Ev> fmt::Debug for TestEffect<Ef, Ev>
where
    Ef: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestEffect")
            .field("request", &self.request)
            .finish()
    }
}
