use crux_core::capability::{CapabilityContext, Operation};
use futures::future::Future;
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum IntervalOperation {
    Start { millis: usize, times: usize },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct IntervalTick {}

impl Operation for IntervalOperation {
    type Output = IntervalTick;
}

#[derive(crux_core::macros::Capability)]
pub struct Interval<Event> {
    context: CapabilityContext<IntervalOperation, Event>,
}

impl<Ev> Interval<Ev>
where
    Ev: Send + 'static,
{
    pub fn new(context: CapabilityContext<IntervalOperation, Ev>) -> Self {
        Self { context }
    }

    pub fn start<F>(&self, millis: usize, times: usize, callback: F)
    where
        F: Fn(IntervalTick) -> Ev + Send + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let mut stream = ctx.stream_from_shell(IntervalOperation::Start { millis, times });

            while let Some(res) = stream.next().await {
                ctx.update_app(callback(res));
            }
        });
    }
}

#[allow(unused)]
pub fn interval(
    interval: &IntervalOperation,
) -> impl futures::Stream<Item = <IntervalOperation as Operation>::Output> {
    let IntervalOperation::Start { millis, times } = interval;
    let times = *times;
    let millis = *millis as u64;

    let (sender, receiver) = async_std::channel::bounded(1);
    spawn(async move {
        let duration = std::time::Duration::from_millis(millis as u64);
        for i in 0..times {
            async_std::task::sleep(duration).await;
            sender.send(IntervalTick {}).await.unwrap();
        }
    });

    receiver
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn<F>(f: F)
where
    F: Future + Send + 'static,
{
    async_std::task::spawn(async {
        f.await;
    });
}

#[cfg(target_arch = "wasm32")]
pub fn spawn<F>(f: F)
where
    F: Future + 'static,
{
    wasm_bindgen_futures::spawn_local(async {
        f.await;
    });
}
