use crux_core::capability::{CapabilityContext, Operation};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum DelayOperation {
    Start { millis: usize },
}

impl Operation for DelayOperation {
    type Output = ();
}

#[derive(crux_core::macros::Capability)]
pub struct Delay<Event> {
    context: CapabilityContext<DelayOperation, Event>,
}

impl<Ev> Delay<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<DelayOperation, Ev>) -> Self {
        Self { context }
    }

    pub fn start(&self, millis: usize, event: Ev)
    where
        Ev: Send,
    {
        self.context.spawn({
            let context = self.context.clone();

            async move {
                context
                    .request_from_shell(DelayOperation::Start { millis })
                    .await;

                context.update_app(event);
            }
        })
    }
}

#[allow(unused)]
#[cfg(target_arch = "wasm32")]
pub async fn delay(delay: &DelayOperation) -> <DelayOperation as Operation>::Output {
    let (sender, receiver) = async_std::channel::bounded(1);
    let DelayOperation::Start { millis } = delay;
    let duration = std::time::Duration::from_millis(*millis as u64);
    wasm_bindgen_futures::spawn_local(async move {
        _delay(duration).await;
        sender.send(()).await.unwrap();
    });
    receiver.recv().await;
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn delay(delay: &DelayOperation) -> <DelayOperation as Operation>::Output {
    let DelayOperation::Start { millis } = delay;
    let duration = std::time::Duration::from_millis(*millis as u64);
    _delay(duration).await;
}

pub async fn _delay(duration: std::time::Duration) -> <DelayOperation as Operation>::Output {
    async_std::task::sleep(duration).await;
}
