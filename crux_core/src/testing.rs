use std::{fmt, rc::Rc};

use crate::{
    capability::CapabilityContext,
    channels::Receiver,
    executor::{executor_and_spawner, QueuingExecutor},
    steps::StepRegistry,
    Request, Step, WithContext,
};

pub struct AppTester<App, Ef>
where
    App: crate::App,
{
    app: App,
    capabilities: App::Capabilities,
    context: Rc<AppContext<Ef, App::Event>>,
}

// TODO: I think this could probably be shared with Core to cut down on a bit of code.
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
    pub fn update(&self, msg: App::Event, model: &mut App::Model) -> Update<Ef, App::Event> {
        self.app.update(msg, model, &self.capabilities);
        self.context.updates()
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
        // Thoughts: all of this shit is _kind_ of an AppContext or CoreContext or something isn't it...?
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

#[derive(Debug)]
// TODO: is this is a shit name?  I feel it might be.
pub struct Update<Ef, Ev> {
    pub effects: Vec<TestEffect<Ef, Ev>>,
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
