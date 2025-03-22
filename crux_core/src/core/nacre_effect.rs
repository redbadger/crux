use super::Effect;
use futures::Stream;
use std::fmt::Debug;
use std::sync::Arc;

pub trait NacreEffect: Debug + Effect + Send + Sized {
    type ShellEffect;
    fn handle<A>(
        self,
        core: Arc<crate::Core<A>>,
    ) -> impl Stream<Item = Vec<Self::ShellEffect>> + Send
    where
        A: crate::App<Effect = Self> + Send + Sync + 'static,
        A::Capabilities: Send + Sync,
        A::Model: Send + Sync;
}
