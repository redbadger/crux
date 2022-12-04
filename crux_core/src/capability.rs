use std::sync::Arc;

use futures::Future;

use crate::{
    channels::{Receiver, Sender},
    Command,
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

pub fn test_capabilities<Caps, App, Ef>() -> (App::Capabilities, Receiver<Command<Ef>>)
where
    Caps: CapabilitiesFactory<App, Ef>,
    App: crate::App,
    App::Event: Send,
    Ef: Send + 'static,
{
    todo!("reinstate this")
    // let (sender, receiver) = channel();
    // (Caps::build(sender), receiver)
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
