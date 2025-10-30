use std::future::Future;
use std::pin::{Pin, pin};

use std::sync::Arc;
use std::task::{Context, Poll};

use crossbeam_channel::Sender;
use futures::channel::mpsc;
use futures::future::Fuse;
use futures::stream::StreamFuture;
use futures::{FutureExt as _, Stream, StreamExt};

use crate::capability::Operation;
use crate::{MaybeSend, Request};

use super::executor::{JoinHandle, Task};

/// Context enabling tasks to communicate with the parent Command,
/// specifically submit effects, events and spawn further tasks
pub struct CommandContext<Effect, Event> {
    pub(crate) effects: Sender<Effect>,
    pub(crate) events: Sender<Event>,
    pub(crate) tasks: Sender<Task>,
    pub(crate) rc: Arc<()>,
}

// derive(Clone) wants Effect and Event to be clone which is not actually necessary
impl<Effect, Event> Clone for CommandContext<Effect, Event> {
    fn clone(&self) -> Self {
        Self {
            effects: self.effects.clone(),
            events: self.events.clone(),
            tasks: self.tasks.clone(),
            rc: self.rc.clone(),
        }
    }
}

impl<Effect, Event> CommandContext<Effect, Event> {
    /// Create a one-off notification to the shell. This method returns immediately.
    #[allow(clippy::missing_panics_doc)]
    pub fn notify_shell<Op>(&self, operation: Op)
    where
        Op: Operation,
        Effect: From<Request<Op>>,
    {
        let request = Request::resolves_never(operation);

        self.effects
            .send(request.into())
            .expect("Command could not send notification, effect channel disconnected");
    }

    /// Create a one-off request for an operation. Returns a future which eventually resolves
    /// with the output of the operation provided by the shell.
    ///
    /// # Cancellation behaviour
    ///
    /// `ShellRequest` futures may never resolve, if the corresponding [`RequestHandle`]
    /// is dropped by the shell. Such cases are detected by the Command and the owning task is aborted.
    /// That is to say - any `.await` point on a `ShellRequest` is a potential abort point for the
    /// enclosing future.
    #[allow(clippy::missing_panics_doc)]
    pub fn request_from_shell<Op>(&self, operation: Op) -> ShellRequest<Op::Output>
    where
        Op: Operation,
        Effect: From<Request<Op>> + MaybeSend + 'static,
    {
        let (output_sender, output_receiver) = mpsc::unbounded();

        let request = Request::resolves_once(operation, move |output| {
            // If the channel is closed, the associated task has been cancelled
            let _ = output_sender.unbounded_send(output);
        });

        let send_request = {
            let effect = request.into();
            let effects = self.effects.clone();
            move || {
                effects
                    .send(effect)
                    .expect("Command could not send request effect, effect channel disconnected");
            }
        };

        ShellRequest::new(Box::new(send_request), output_receiver)
    }

    /// Create a stream request for an operation. Returns a stream producing the
    /// with the output of the operation every time it is provided by the shell.
    ///
    /// # Cancellation behaviour
    ///
    /// `ShellStream` futures may never resolve, if the corresponding [`RequestHandle`]
    /// is dropped by the shell. Such cases are detected by the Command and the owning task is aborted.
    /// That is to say - any `.await` point on a `ShellRequest` is a potential abort point for the
    /// enclosing future.
    #[allow(clippy::missing_panics_doc)]
    pub fn stream_from_shell<Op>(&self, operation: Op) -> ShellStream<Op::Output>
    where
        Op: Operation,
        Effect: From<Request<Op>> + MaybeSend + 'static,
    {
        let (output_sender, output_receiver) = mpsc::unbounded();

        let request = Request::resolves_many_times(operation, move |output| {
            output_sender.unbounded_send(output).map_err(|_| ())?;

            // TODO: revisit the error handling in here
            Ok(())
        });

        let send_request = {
            let effect = request.into();
            let effects = self.effects.clone();
            move || {
                effects
                    .send(effect)
                    .expect("Command could not send stream effect, effect channel disconnected");
            }
        };

        ShellStream::new(send_request, output_receiver)
    }

    /// Send an event which should be handed to the update function. This is used to communicate the result
    /// (or a sequence of results) of a command back to the app so that state can be updated accordingly
    #[allow(clippy::missing_panics_doc)]
    pub fn send_event(&self, event: Event) {
        self.events
            .send(event)
            .expect("Command could not send event, event channel disconnected");
    }

    /// Spawn a new task within the same command. The task will execute concurrently with other tasks within the
    /// command until it either concludes, is aborted, or until the parent command is aborted.
    ///
    /// Returns a `JoinHandle` which can be used as a future to await the completion of the task. It can also
    /// be used to abort the task.
    #[allow(clippy::missing_panics_doc)]
    pub fn spawn<F, Fut>(&self, make_future: F) -> JoinHandle
    where
        F: FnOnce(CommandContext<Effect, Event>) -> Fut,
        Fut: Future<Output = ()> + MaybeSend + 'static,
    {
        let (sender, receiver) = crossbeam_channel::unbounded();

        let ctx = self.clone();
        let future = make_future(ctx);

        let task = Task {
            finished: Arc::default(),
            aborted: Arc::default(),
            #[cfg(not(feature = "unsync"))]
            future: future.boxed(),
            #[cfg(feature = "unsync")]
            future: future.boxed_local(),
            join_handle_wakers: receiver,
        };

        let handle = JoinHandle {
            finished: task.finished.clone(),
            aborted: task.aborted.clone(),
            register_waker: sender,
        };

        self.tasks
            .send(task)
            .expect("Command could not spawn task, tasks channel disconnected");

        handle
    }
}

pub trait SendRequest: FnOnce() + MaybeSend {}

impl<T> SendRequest for T where T: FnOnce() + MaybeSend {}

pub enum ShellStream<T: Unpin + MaybeSend> {
    ReadyToSend(Box<dyn SendRequest>, mpsc::UnboundedReceiver<T>),
    Sent(mpsc::UnboundedReceiver<T>),
}

impl<T: Unpin + MaybeSend> ShellStream<T> {
    fn new(
        send_request: impl FnOnce() + MaybeSend + 'static,
        output_receiver: mpsc::UnboundedReceiver<T>,
    ) -> Self {
        ShellStream::ReadyToSend(Box::new(send_request), output_receiver)
    }

    fn send(&mut self) {
        // Since neither part is Clone, we'll need to do an Indiana Jones

        // 1. take items out of self
        let dummy = ShellStream::Sent(mpsc::unbounded().1);
        let ShellStream::ReadyToSend(send_request, output_receiver) =
            std::mem::replace(self, dummy)
        else {
            unreachable!();
        };

        // 2. replace self with with a Sent using the original receiver
        *self = ShellStream::Sent(output_receiver);

        send_request();
    }
}

impl<T: Unpin + MaybeSend> Stream for ShellStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match *self {
            ShellStream::ReadyToSend(_, ref mut output_receiver) => {
                let poll = pin!(output_receiver).poll_next(cx);
                assert!(matches!(poll, Poll::Pending)); // we have not sent the request yet

                self.send();

                Poll::Pending
            }
            ShellStream::Sent(ref mut output_receiver) => pin!(output_receiver).poll_next(cx),
        }
    }
}

pub struct ShellRequest<T: Unpin + MaybeSend> {
    inner: Fuse<StreamFuture<ShellStream<T>>>,
}

impl<T: Unpin + MaybeSend + 'static> ShellRequest<T> {
    fn new(
        send_request: impl FnOnce() + MaybeSend + 'static,
        output_receiver: mpsc::UnboundedReceiver<T>,
    ) -> Self {
        let inner = ShellStream::new(send_request, output_receiver)
            .into_future()
            .fuse();

        Self { inner }
    }
}

impl<T: Unpin + MaybeSend> Future for ShellRequest<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.inner.poll_unpin(cx) {
            Poll::Ready((Some(output), _rest)) => Poll::Ready(output),
            Poll::Ready((None, _rest)) => Poll::Pending,
            Poll::Pending => Poll::Pending,
        }
    }
}
