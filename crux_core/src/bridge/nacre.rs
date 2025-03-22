use crate::bridge::{NacreTrait, Shell};
use crate::{Core, Middleware, NacreEffect, WithContext};
use futures::StreamExt;
use std::future::Future;
use std::sync::Arc;
use streamunordered::{StreamUnordered, StreamYield};

pub struct Nacre<A>
where
    A: crate::App + Send + Sync + 'static,
    A::Effect: NacreEffect,
    A::Capabilities: WithContext<A::Event, A::Effect> + Send + Sync,
    A::Model: Send + Sync,
    <A::Effect as NacreEffect>::ShellEffect: Send,
{
    core: Arc<crate::Core<A>>,
    sender: async_std::channel::Sender<Vec<<A::Effect as NacreEffect>::ShellEffect>>,
}

pub type ShellEffects<A> = Vec<<<A as crate::App>::Effect as NacreEffect>::ShellEffect>;

impl<A> Nacre<A>
where
    A: crate::App + Send + Sync + 'static,
    A::Effect: NacreEffect,
    A::Capabilities: WithContext<A::Event, A::Effect> + Send + Sync,
    A::Model: Send + Sync,
    <A::Effect as NacreEffect>::ShellEffect: Send,
{
    pub fn new(
        sender: async_std::channel::Sender<Vec<<A::Effect as NacreEffect>::ShellEffect>>,
    ) -> Self {
        Self {
            core: Arc::new(Core::new()),
            sender,
        }
    }

    fn process(&self, effects: impl Iterator<Item = A::Effect>) {
        let effects = effects.collect::<Vec<_>>();
        let core = self.core.clone();
        let sender = self.sender.clone();
        self.spawn(async move {
            let all_streams = effects.into_iter().map(|effect| {
                let core = core.clone();
                effect.handle(core)
            });
            let mut streams = StreamUnordered::from_iter(all_streams);

            while let Some((yielded, _)) = streams.next().await {
                match yielded {
                    StreamYield::Item(effects) => {
                        sender.send(effects).await.unwrap();
                    }
                    StreamYield::Finished(_stream) => {
                        // TODO: remove the stream
                        // stream.remove(streams);
                    }
                }
            }
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn<F>(&self, f: F)
    where
        F: Future + Send + 'static,
        F::Output: Send,
    {
        async_std::task::spawn(async {
            f.await;
        });
    }

    #[cfg(target_arch = "wasm32")]
    pub fn spawn<F>(&self, f: F)
    where
        F: Future + Send + 'static,
        F::Output: Send,
    {
        wasm_bindgen_futures::spawn_local(async {
            f.await;
        });
    }
}

pub struct NacreBridge<A>
where
    A: crate::App + Send + Sync + 'static,
    A::Effect: NacreEffect,
    A::Capabilities: WithContext<A::Event, A::Effect> + Send + Sync,
    A::Model: Send + Sync,
    <A::Effect as NacreEffect>::ShellEffect: Send,
{
    nacre: Nacre<A>,
    shell: Arc<dyn Shell>,
}

impl<A> NacreBridge<A>
where
    A: crate::App + Send + Sync + 'static,
    A::Effect: NacreEffect,
    A::Capabilities: WithContext<A::Event, A::Effect> + Send + Sync,
    A::Model: Send + Sync,
    <A::Effect as NacreEffect>::ShellEffect: Send,
{
    pub fn new(
        sender: async_std::channel::Sender<Vec<<A::Effect as NacreEffect>::ShellEffect>>,
        shell: Arc<dyn Shell>,
    ) -> Self {
        Self {
            nacre: Nacre::new(sender),
            shell,
        }
    }
}

impl<A> NacreTrait<A> for NacreBridge<A>
where
    A: crate::App + Send + Sync + 'static,
    A::Effect: NacreEffect,
    A::Capabilities: WithContext<A::Event, A::Effect> + Send + Sync,
    A::Model: Send + Sync,
    <A::Effect as NacreEffect>::ShellEffect: Send,
{
    fn register_callback(
        &self,
        receiver: async_std::channel::Receiver<ShellEffects<A>>,
        cb: impl Fn(ShellEffects<A>) -> Vec<u8> + Send + 'static,
    ) {
        let shell = self.shell.clone();
        self.nacre.spawn(async move {
            while let Ok(effect) = receiver.recv().await {
                let bytes = cb(effect);
                shell.handle_effects(bytes);
            }
        });
    }
}

impl<A> Middleware for NacreBridge<A>
where
    A: crate::App + Send + Sync + 'static,
    A::Effect: NacreEffect,
    A::Capabilities: WithContext<A::Event, A::Effect> + Send + Sync,
    A::Model: Send + Sync,
    <A::Effect as NacreEffect>::ShellEffect: Send,
{
    type App = A;

    fn process_event(&self, event: A::Event) -> impl Iterator<Item = A::Effect> {
        self.nacre.process_event(event)
    }

    fn process_effects(&self) -> impl Iterator<Item = A::Effect> {
        self.nacre.process_effects()
    }

    fn view(&self) -> A::ViewModel {
        self.nacre.view()
    }
}

impl<A> Middleware for Nacre<A>
where
    A: crate::App + Send + Sync + 'static,
    A::Effect: NacreEffect,
    A::Capabilities: WithContext<A::Event, A::Effect> + Send + Sync,
    A::Model: Send + Sync,
    <A::Effect as NacreEffect>::ShellEffect: Send,
{
    type App = A;

    fn process_event(&self, event: A::Event) -> impl Iterator<Item = A::Effect> {
        self.process(self.core.process_event(event));
        vec![].into_iter()
    }

    fn process_effects(&self) -> impl Iterator<Item = A::Effect> {
        self.process(self.core.process_effects());
        vec![].into_iter()
    }

    fn view(&self) -> A::ViewModel {
        self.core.view()
    }
}
