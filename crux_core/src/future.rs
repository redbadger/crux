use std::{
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

use futures::Future;

use crate::Step;

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

        match shared_state.result.take() {
            Some(result) => Poll::Ready(result),
            None => {
                shared_state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl<Op, Ev> crate::capability::CapabilityContext<Op, Ev>
where
    Op: crate::capability::Operation,
    Ev: 'static,
{
    pub fn request_effect(&self, operation: Op) -> EffectFuture<Op::Output> {
        let shared_state = Arc::new(Mutex::new(SharedState {
            result: None,
            waker: None,
        }));

        let callback_shared_state = shared_state.clone();
        self.advance(Step::new(operation, move |bytes| {
            let mut shared_state = callback_shared_state.lock().unwrap();
            shared_state.result = Some(bcs::from_bytes(bytes).unwrap());
            if let Some(waker) = shared_state.waker.take() {
                waker.wake()
            }
        }));

        EffectFuture { shared_state }
    }
}
