use std::{fmt, marker::PhantomData, rc::Rc, sync::Arc};

use futures::Future;

use crate::{
    capability::CapabilityContext,
    channels::{Receiver, Sender},
    continuations::ContinuationStore,
    executor::{executor_and_spawner, Executor},
    CapabilitiesFactory, Command, Request,
};

pub struct AppTester<App, Ef>
where
    App: crate::App,
{
    app: App,
    capabilities: App::Capabilities,
    commands: Receiver<Command<Ef>>,
    events: Rc<Receiver<App::Event>>,
    executor: Rc<Executor>,
    continuation_store: Rc<ContinuationStore>,
}

impl<App, Ef> AppTester<App, Ef>
where
    App: crate::App,
{
    pub fn update(&self, msg: App::Event, model: &mut App::Model) -> Vec<TestEffect<Ef>> {
        self.app.update(msg, model, &self.capabilities);

        self.effects()
    }

    fn effects(&self) -> Vec<TestEffect<Ef>> {
        self.executor.run_all();
        self.commands
            .drain()
            .map(|cmd| {
                let request = self.continuation_store.pause(cmd);
                TestEffect {
                    request,
                    store: self.continuation_store.clone(),
                    executor: self.executor.clone(),
                }
            })
            .collect()
    }

    pub fn events(&self) -> Vec<App::Event> {
        self.events.drain().collect()
    }
}

impl<App, Ef> Default for AppTester<App, Ef>
where
    App: crate::App,
    App::Capabilities: CapabilitiesFactory<App, Ef>,
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
            capabilities: App::Capabilities::build(capability_context),
            commands,
            events: Rc::new(events),
            executor: Rc::new(executor),
            continuation_store: Rc::new(ContinuationStore::default()),
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

pub struct TestEffect<Ef> {
    request: Request<Ef>,
    store: Rc<ContinuationStore>,
    executor: Rc<Executor>,
}

impl<Ef> TestEffect<Ef> {
    pub fn resolve<T>(&self, result: &T)
    where
        T: serde::ser::Serialize,
    {
        self.store.resume(
            self.request.uuid.as_slice(),
            &bcs::to_bytes(result).unwrap(),
        );

        // TODO: decide if this is a good idea...
        self.executor.run_all();
    }
}

impl<Ef> AsRef<Ef> for TestEffect<Ef> {
    fn as_ref(&self) -> &Ef {
        &self.request.effect
    }
}

impl<Ef> PartialEq<Ef> for TestEffect<Ef>
where
    Ef: PartialEq,
{
    fn eq(&self, other: &Ef) -> bool {
        self.request.effect == *other
    }
}

impl<Ef> fmt::Debug for TestEffect<Ef>
where
    Ef: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestEffect")
            .field("request", &self.request)
            .finish()
    }
}
