use std::{fmt, marker::PhantomData, rc::Rc, sync::Arc};

use futures::Future;

use crate::{
    channels::{Receiver, Sender},
    continuations::ContinuationStore,
    executor::{executor_and_spawner, Executor},
    Command, Request,
};

// TODO docs!
pub trait Capability<Ev> {
    type MappedSelf<MappedEv>;

    fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
    where
        F: Fn(NewEvent) -> Ev + Send + Sync + Copy + 'static,
        Ev: 'static,
        NewEvent: 'static;
}

pub trait CapabilitiesFactory<App, Ef>
where
    App: crate::App,
{
    fn build(context: CapabilityContext<Ef, App::Event>) -> App::Capabilities;
}

pub struct CapabilityContext<Ef, Event> {
    inner: std::sync::Arc<ContextInner<Ef, Event>>,
}

struct ContextInner<Ef, Event> {
    command_sender: Sender<Command<Ef>>,
    event_sender: Sender<Event>,
    spawner: crate::executor::Spawner,
}

impl<Ef, Ev> Clone for CapabilityContext<Ef, Ev> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<Ef, Ev> CapabilityContext<Ef, Ev>
where
    Ef: 'static,
    Ev: 'static,
{
    pub(crate) fn new(
        command_sender: Sender<Command<Ef>>,
        event_sender: Sender<Ev>,
        spawner: crate::executor::Spawner,
    ) -> Self {
        let inner = Arc::new(ContextInner {
            command_sender,
            event_sender,
            spawner,
        });

        CapabilityContext { inner }
    }

    pub(crate) fn run_command(&self, cmd: Command<Ef>) {
        self.inner.command_sender.send(cmd);
    }

    pub fn spawn(&self, f: impl Future<Output = ()> + 'static + Send) {
        self.inner.spawner.spawn(f);
    }

    pub fn send_event(&self, event: Ev) {
        self.inner.event_sender.send(event);
    }

    pub fn map_effect<NewEf, F>(&self, func: F) -> CapabilityContext<NewEf, Ev>
    where
        F: Fn(NewEf) -> Ef + Sync + Send + Copy + 'static,
        NewEf: 'static,
    {
        let inner = Arc::new(ContextInner {
            command_sender: self.inner.command_sender.map_effect(func),
            event_sender: self.inner.event_sender.clone(),
            spawner: self.inner.spawner.clone(),
        });

        CapabilityContext { inner }
    }

    pub fn map_event<NewEv, F>(&self, func: F) -> CapabilityContext<Ef, NewEv>
    where
        F: Fn(NewEv) -> Ev + Sync + Send + 'static,
        NewEv: 'static,
    {
        let inner = Arc::new(ContextInner {
            command_sender: self.inner.command_sender.clone(),
            event_sender: self.inner.event_sender.map_input(func),
            spawner: self.inner.spawner.clone(),
        });

        CapabilityContext { inner }
    }
}

// TODO: Could this be "TestCore" or someshit?
pub struct CapabilitiesTestHelper<App, Ef>
where
    App: crate::App,
{
    capabilities: App::Capabilities,
    commands: Receiver<Command<Ef>>,
    events: Rc<Receiver<App::Event>>,
    executor: Rc<Executor>,
    continuation_store: Rc<ContinuationStore>,
    phantom: PhantomData<fn(App) -> Ef>,
}

impl<App, Ef> CapabilitiesTestHelper<App, Ef>
where
    App: crate::App,
{
    pub fn effects(&self) -> Vec<TestEffect<Ef>> {
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

impl<App, Ef> Default for CapabilitiesTestHelper<App, Ef>
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
            capabilities: App::Capabilities::build(capability_context),
            commands,
            events: Rc::new(events),
            executor: Rc::new(executor),
            continuation_store: Rc::new(ContinuationStore::default()),
            phantom: PhantomData,
        }
    }
}

impl<App, Ef> AsRef<App::Capabilities> for CapabilitiesTestHelper<App, Ef>
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

#[cfg(test)]
mod tests {
    use static_assertions::assert_impl_all;

    use super::*;

    #[allow(dead_code)]
    enum Effect {}

    #[allow(dead_code)]
    enum Event {}

    assert_impl_all!(CapabilityContext<Effect, Event>: Send, Sync);
}
