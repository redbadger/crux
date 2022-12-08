use std::sync::Arc;

use futures::Future;

use crate::{channels::Sender, Step};

pub trait Operation: serde::Serialize + Send + 'static {
    type Output: serde::de::DeserializeOwned + Send + 'static;
}

// TODO docs!
pub trait Capability<Ev> {
    type MappedSelf<MappedEv>;

    fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
    where
        F: Fn(NewEvent) -> Ev + Send + Sync + Copy + 'static,
        Ev: 'static,
        NewEvent: 'static;
}

pub trait WithContext<App, Ef>
where
    App: crate::App,
{
    fn new_with_context(context: CapabilityContext<Ef, App::Event>) -> App::Capabilities;
}

pub struct CapabilityContext<Ef, Event> {
    inner: std::sync::Arc<ContextInner<Ef, Event>>,
}

struct ContextInner<Ef, Event> {
    steps: Sender<Step<Ef>>,
    events: Sender<Event>,
    spawner: crate::executor::Spawner,
}

impl<Ef, Ev> Clone for CapabilityContext<Ef, Ev> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T, Ev> CapabilityContext<T, Ev>
where
    T: 'static,
    Ev: 'static,
{
    pub(crate) fn new(
        steps: Sender<Step<T>>,
        events: Sender<Ev>,
        spawner: crate::executor::Spawner,
    ) -> Self {
        let inner = Arc::new(ContextInner {
            steps,
            events,
            spawner,
        });

        CapabilityContext { inner }
    }

    pub fn spawn(&self, f: impl Future<Output = ()> + 'static + Send) {
        self.inner.spawner.spawn(f);
    }

    pub fn notify_shell(&self, operation: T) {
        self.inner.steps.send(Step::once(operation));
    }

    pub fn update_app(&self, event: Ev) {
        self.inner.events.send(event);
    }

    pub fn with_effect<OtherT, F>(&self, func: F) -> CapabilityContext<OtherT, Ev>
    where
        F: Fn(OtherT) -> T + Sync + Send + Copy + 'static,
        OtherT: 'static,
    {
        let inner = Arc::new(ContextInner {
            steps: self.inner.steps.map_effect(func),
            events: self.inner.events.clone(),
            spawner: self.inner.spawner.clone(),
        });

        CapabilityContext { inner }
    }

    pub fn map_event<NewEv, F>(&self, func: F) -> CapabilityContext<T, NewEv>
    where
        F: Fn(NewEv) -> Ev + Sync + Send + 'static,
        NewEv: 'static,
    {
        let inner = Arc::new(ContextInner {
            steps: self.inner.steps.clone(),
            events: self.inner.events.map_input(func),
            spawner: self.inner.spawner.clone(),
        });

        CapabilityContext { inner }
    }

    pub(crate) fn send_step(&self, step: Step<T>) {
        self.inner.steps.send(step);
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
