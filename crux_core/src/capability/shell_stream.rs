use std::{
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

use futures::Stream;

use super::{channel, channel::Receiver};
use crate::core::Request;

pub struct ShellStream<T> {
    shared_state: Arc<Mutex<SharedState<T>>>,
}

struct SharedState<T> {
    receiver: Receiver<T>,
    waker: Option<Waker>,
    send_request: Option<Box<dyn FnOnce() + Send + 'static>>,
}

impl<T> Stream for ShellStream<T> {
    type Item = T;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut shared_state = self.shared_state.lock().unwrap();

        if let Some(send_request) = shared_state.send_request.take() {
            send_request();
        }

        match shared_state.receiver.try_receive() {
            Ok(Some(next)) => Poll::Ready(Some(next)),
            Ok(None) => {
                shared_state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
            Err(_) => Poll::Ready(None),
        }
    }
}

impl<Op, Ev> crate::capability::CapabilityContext<Op, Ev>
where
    Op: crate::capability::Operation,
    Ev: 'static,
{
    /// Send an effect request to the shell, expecting a stream of responses
    pub fn stream_from_shell(&self, operation: Op) -> ShellStream<Op::Output> {
        let (sender, receiver) = channel();
        let shared_state = Arc::new(Mutex::new(SharedState {
            receiver,
            waker: None,
            send_request: None,
        }));

        // Our callback holds a weak pointer so the channel can be freed
        // whenever the associated task ends.
        let callback_shared_state = Arc::downgrade(&shared_state);

        let request = Request::resolves_many_times(operation, move |result| {
            let Some(shared_state) = callback_shared_state.upgrade() else {
                // Let the caller know that the associated task has finished.
                return Err(());
            };

            let mut shared_state = shared_state.lock().unwrap();

            sender.send(result);
            if let Some(waker) = shared_state.waker.take() {
                waker.wake();
            }

            Ok(())
        });

        // Put a callback into our shared_state so that we only send
        // our request to the shell when the stream is first polled.
        let send_req_context = self.clone();
        let send_request = move || send_req_context.send_request(request);
        shared_state.lock().unwrap().send_request = Some(Box::new(send_request));

        ShellStream { shared_state }
    }
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;

    use crate::capability::{channel, executor_and_spawner, CapabilityContext, Operation};

    #[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, Debug)]
    struct TestOperation;

    impl Operation for TestOperation {
        type Output = Option<Done>;
    }

    #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
    struct Done;

    #[test]
    fn test_shell_stream() {
        let (request_sender, requests) = channel();
        let (event_sender, events) = channel::<()>();
        let (executor, spawner) = executor_and_spawner();
        let capability_context =
            CapabilityContext::new(request_sender, event_sender.clone(), spawner.clone());

        let mut stream = capability_context.stream_from_shell(TestOperation);

        // The stream hasn't been polled so we shouldn't have any requests.
        assert_matches!(requests.receive(), None);
        assert_matches!(events.receive(), None);

        // It also shouldn't have spawned anything so check that
        executor.run_all();
        assert_matches!(requests.receive(), None);
        assert_matches!(events.receive(), None);

        spawner.spawn(async move {
            use futures::StreamExt;
            while let Some(maybe_done) = stream.next().await {
                event_sender.send(());
                if maybe_done.is_some() {
                    break;
                }
            }
        });

        // We still shouldn't have any requests
        assert_matches!(requests.receive(), None);
        assert_matches!(events.receive(), None);

        executor.run_all();
        let mut request = requests.receive().expect("we should have a request here");

        assert_matches!(requests.receive(), None);
        assert_matches!(events.receive(), None);

        request.resolve(None).unwrap();

        executor.run_all();

        // We should have one event
        assert_matches!(requests.receive(), None);
        assert_matches!(events.receive(), Some(()));
        assert_matches!(events.receive(), None);

        // Resolve it a few more times and then finish.
        request.resolve(None).unwrap();
        request.resolve(None).unwrap();
        request.resolve(Some(Done)).unwrap();
        executor.run_all();

        // We should have three events
        assert_matches!(requests.receive(), None);
        assert_matches!(events.receive(), Some(()));
        assert_matches!(events.receive(), Some(()));
        assert_matches!(events.receive(), Some(()));
        assert_matches!(events.receive(), None);

        // The next resolve should error as we've terminated the task
        request
            .resolve(None)
            .expect_err("resolving a finished task should error");
    }
}
