//! Async support for implementing capabilities
//!
use std::{
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

use futures::Future;

use crate::Request;

pub struct ShellRequest<T> {
    shared_state: Arc<Mutex<SharedState<T>>>,
}

#[cfg(test)]
impl ShellRequest<()> {
    pub(crate) fn new() -> Self {
        Self {
            shared_state: Arc::new(Mutex::new(SharedState {
                result: None,
                waker: None,
                send_request: None,
            })),
        }
    }
}

// State shared between the ShellRequest future itself and the
// Request's resolve callback. The resolve callback is responsible
// for advancing the state from Pending to Complete
//
// FIXME this should be a tri-state enum instead:
// - ReadyToSend(Box<dyn FnOnce() + Send + 'static>)
// - Pending(Waker)
// - Complete(T)
struct SharedState<T> {
    // the effect's output
    result: Option<T>,
    send_request: Option<Box<dyn FnOnce() + Send + 'static>>,
    waker: Option<Waker>,
}

impl<T> Future for ShellRequest<T> {
    type Output = T;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut shared_state = self.shared_state.lock().unwrap();

        // If there's still a request to send, take it and send it
        if let Some(send_request) = shared_state.send_request.take() {
            send_request();
        }

        // If a result has been delivered, we're ready to continue
        // Else we're pending with the waker from context
        match shared_state.result.take() {
            Some(result) => Poll::Ready(result),
            None => {
                let cloned_waker = cx.waker().clone();
                shared_state.waker = Some(cloned_waker);
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
    /// Send an effect request to the shell, expecting an output. The
    /// provided `operation` describes the effect input in a serialisable fashion,
    /// and must implement the [`Operation`](crate::capability::Operation) trait to declare the expected
    /// output type.
    ///
    /// `request_from_shell` returns a future of the output, which can be
    /// `await`ed. You should only call this method inside an async task
    /// created with [`CapabilityContext::spawn`](crate::capability::CapabilityContext::spawn).
    pub fn request_from_shell(&self, operation: Op) -> ShellRequest<Op::Output> {
        let shared_state = Arc::new(Mutex::new(SharedState {
            result: None,
            waker: None,
            send_request: None,
        }));

        // Our callback holds a weak pointer to avoid circular references
        // from shared_state -> send_request -> request -> shared_state
        let callback_shared_state = Arc::downgrade(&shared_state);

        // used in docs/internals/runtime.md
        // ANCHOR: resolve
        let request = Request::resolves_once(operation, move |result| {
            let Some(shared_state) = callback_shared_state.upgrade() else {
                // The ShellRequest was dropped before we were called, so just
                // do nothing.
                return;
            };

            let mut shared_state = shared_state.lock().unwrap();

            // Attach the result to the shared state of the future
            shared_state.result = Some(result);
            // Signal the executor to wake the task holding this future
            if let Some(waker) = shared_state.waker.take() {
                waker.wake()
            }
        });
        // ANCHOR_END: resolve

        // Send the request on the next poll of the ShellRequest future
        let send_req_context = self.clone();
        let send_request = move || send_req_context.send_request(request);

        shared_state.lock().unwrap().send_request = Some(Box::new(send_request));

        ShellRequest { shared_state }
    }
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;

    use crate::capability::{channel, executor_and_spawner, CapabilityContext, Operation};

    #[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, Debug)]
    struct TestOperation;

    impl Operation for TestOperation {
        type Output = ();
    }

    #[test]
    fn test_effect_future() {
        let (request_sender, requests) = channel();
        let (event_sender, events) = channel::<()>();
        let (executor, spawner) = executor_and_spawner();
        let capability_context =
            CapabilityContext::new(request_sender, event_sender.clone(), spawner.clone());

        let future = capability_context.request_from_shell(TestOperation);

        // The future hasn't been awaited so we shouldn't have any requests.
        assert_matches!(requests.receive(), None);
        assert_matches!(events.receive(), None);

        // It also shouldn't have spawned anything so check that
        executor.run_all();
        assert_matches!(requests.receive(), None);
        assert_matches!(events.receive(), None);

        spawner.spawn(async move {
            future.await;
            event_sender.send(());
        });

        // We still shouldn't have any requests
        assert_matches!(requests.receive(), None);
        assert_matches!(events.receive(), None);

        executor.run_all();
        let mut request = requests.receive().expect("we should have a request here");
        assert_matches!(requests.receive(), None);
        assert_matches!(events.receive(), None);

        request.resolve(()).expect("request should resolve");

        assert_matches!(requests.receive(), None);
        assert_matches!(events.receive(), None);

        executor.run_all();
        assert_matches!(requests.receive(), None);
        assert_matches!(events.receive(), Some(()));
        assert_matches!(events.receive(), None);
    }
}
