use std::{
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

use futures::Future;

use crate::Command;

pub struct EffectFuture<T> {
    shared_state: Arc<Mutex<SharedState<T>>>,
}

struct SharedState<T> {
    result: Option<T>,
    waker: Option<Waker>,
}

impl<T> Future for EffectFuture<T> {
    type Output = T;

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
    // Think about this name somewhat...
    pub fn effect(&self, ef: Ef) -> EffectFuture<Ef::Response> {
        let shared_state = Arc::new(Mutex::new(SharedState {
            result: None,
            waker: None,
        }));

        let callback_shared_state = shared_state.clone();
        self.run_command(Command::new_continuation(ef, move |result| {
            let mut shared_state = callback_shared_state.lock().unwrap();
            shared_state.result = Some(result);
            if let Some(waker) = shared_state.waker.take() {
                waker.wake()
            }
        }));

        EffectFuture { shared_state }
    }
}
