use std::{
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

use futures::Future;

use crate::{capability::CapabilityContext, Command};

pub struct EffectFuture {
    shared_state: Arc<Mutex<SharedState>>,
}

struct SharedState {
    result: Option<Vec<u8>>,
    waker: Option<Waker>,
}

impl Future for EffectFuture {
    type Output = Vec<u8>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut shared_state = self.shared_state.lock().unwrap();

        if let Some(result) = shared_state.result.take() {
            return Poll::Ready(result);
        }

        shared_state.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl<Ef, Ev> crate::capability::CapabilityContext<Ef, Ev>
where
    Ef: crate::Effect,
    Ev: 'static,
{
    pub async fn effect(&self, effect: Ef) -> Ef::Response {
        let bytes = effect_future(self, effect).await;

        bcs::from_bytes(&bytes).unwrap()
    }
}

fn effect_future<Ef, Ev>(ctx: &CapabilityContext<Ef, Ev>, effect: Ef) -> EffectFuture
where
    Ef: crate::Effect,
    Ev: 'static,
{
    let shared_state = Arc::new(Mutex::new(SharedState {
        result: None,
        waker: None,
    }));

    let callback_shared_state = shared_state.clone();
    ctx.run_command(Command::new(effect, move |result| {
        let mut shared_state = callback_shared_state.lock().unwrap();
        shared_state.result = Some(result);
        if let Some(waker) = shared_state.waker.take() {
            waker.wake()
        }
    }));

    EffectFuture { shared_state }
}
