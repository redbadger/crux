use std::{
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

use futures::Stream;

use crate::{
    channels::{channel, Receiver},
    steps::Step,
};

pub struct ShellStream<T> {
    shared_state: Arc<Mutex<SharedState<T>>>,
}

struct SharedState<T> {
    receiver: Receiver<T>,
    waker: Option<Waker>,
    send_step: Option<Box<dyn FnOnce() + Send + 'static>>,
}

impl<T> Stream for ShellStream<T> {
    type Item = T;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut shared_state = self.shared_state.lock().unwrap();

        if let Some(send_step) = shared_state.send_step.take() {
            send_step();
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
            send_step: None,
        }));

        // Our callback holds a weak pointer so the channel can be freed
        // whenever the associated task ends.
        let callback_shared_state = Arc::downgrade(&shared_state);

        let step = Step::resolves_many_times(operation, move |result| {
            let Some(shared_state) = callback_shared_state.upgrade() else {
                // Let the StepRegistry know that the associated task has finished.
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
        let send_step_context = self.clone();
        let send_step = move || send_step_context.send_step(step);
        shared_state.lock().unwrap().send_step = Some(Box::new(send_step));

        ShellStream { shared_state }
    }
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;

    use crate::{
        capability::{CapabilityContext, Operation},
        channels::channel,
        executor::executor_and_spawner,
        steps::Step,
    };

    #[derive(serde::Serialize, PartialEq, Eq, Debug)]
    struct TestOperation;

    impl Operation for TestOperation {
        type Output = Option<Done>;
    }

    #[derive(serde::Deserialize, PartialEq, Eq, Debug)]
    struct Done;

    #[test]
    fn test_shell_stream() {
        let (step_sender, steps) = channel();
        let (event_sender, events) = channel::<()>();
        let (executor, spawner) = executor_and_spawner();
        let capability_context =
            CapabilityContext::new(step_sender, event_sender.clone(), spawner.clone());

        let mut stream = capability_context.stream_from_shell(TestOperation);

        // The stream hasn't been polled so we shouldn't have any steps.
        assert_matches!(steps.receive(), None);
        assert_matches!(events.receive(), None);

        // It also shouldn't have spawned anything so check that
        executor.run_all();
        assert_matches!(steps.receive(), None);
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

        // We still shouldn't have any steps
        assert_matches!(steps.receive(), None);
        assert_matches!(events.receive(), None);

        executor.run_all();
        let step = steps.receive().expect("we should have a step here");

        let step = &match step {
            Step::Many(s) => s,
            _ => panic!("expected a Step::ResolveMany"),
        };

        assert_matches!(steps.receive(), None);
        assert_matches!(events.receive(), None);

        step.resolve(None).unwrap();

        executor.run_all();

        // We should have one event
        assert_matches!(steps.receive(), None);
        assert_matches!(events.receive(), Some(()));
        assert_matches!(events.receive(), None);

        // Resolve it a few more times and then finish.
        step.resolve(None).unwrap();
        step.resolve(None).unwrap();
        step.resolve(Some(Done)).unwrap();
        executor.run_all();

        // We should have three events
        assert_matches!(steps.receive(), None);
        assert_matches!(events.receive(), Some(()));
        assert_matches!(events.receive(), Some(()));
        assert_matches!(events.receive(), Some(()));
        assert_matches!(events.receive(), None);

        // The next resolve should error as we've terminated the task
        step.resolve(None)
            .expect_err("resolving a finished task should error")
    }
}
