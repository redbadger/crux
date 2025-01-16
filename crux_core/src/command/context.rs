use std::future::Future;
use std::ops::DerefMut as _;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use crossbeam_channel::{Receiver, Sender, TryRecvError};

use futures::task::AtomicWaker;
use futures::{FutureExt as _, Stream};

use crate::capability::Operation;
use crate::Request;

use super::executor::JoinHandle;
use super::executor::Task;

/// Context enabling tasks to communicate with the parent Command,
/// specifically submit effects, events and spawn further tasks
pub struct CommandContext<Effect, Event> {
    pub(crate) effects: Sender<Effect>,
    pub(crate) events: Sender<Event>,
    pub(crate) tasks: Sender<Task>,
}

// derive(Clone) wants Effect and Event to be clone which is not actually necessary
impl<Effect, Event> Clone for CommandContext<Effect, Event> {
    fn clone(&self) -> Self {
        Self {
            effects: self.effects.clone(),
            events: self.events.clone(),
            tasks: self.tasks.clone(),
        }
    }
}

impl<Effect, Event> CommandContext<Effect, Event> {
    /// Create a one-off notification to the shell. This method returns immediately.
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
    pub fn request_from_shell<Op>(&self, operation: Op) -> ShellRequest<Op::Output>
    where
        Op: Operation,
        Effect: From<Request<Op>> + Send + 'static,
    {
        let (output_sender, output_receiver) = crossbeam_channel::bounded(1);
        let shared_waker = Arc::new(AtomicWaker::new());

        let request = Request::resolves_once(operation, {
            let waker = shared_waker.clone();

            move |output| {
                // If the channel is closed, the associated task has been cancelled
                if output_sender.send(output).is_ok() {
                    waker.wake();
                }
            }
        });

        let send_request = {
            let effect = request.into();
            let effects = self.effects.clone();
            move || {
                effects
                    .send(effect)
                    .expect("Command could not send request effect, effect channel disconnected")
            }
        };

        ShellRequest::ReadyToSend(Box::new(send_request), shared_waker, output_receiver)
    }

    /// Create a stream request for an operation. Returns a stream producing the
    /// with the output of the operation every time it is provided by the shell.
    pub fn stream_from_shell<Op>(&self, operation: Op) -> ShellStream<Op::Output>
    where
        Op: Operation,
        Effect: From<Request<Op>> + Send + 'static,
    {
        let (output_sender, output_receiver) = crossbeam_channel::unbounded();
        let shared_waker = Arc::new(AtomicWaker::new());

        let request = Request::resolves_many_times(operation, {
            let waker = shared_waker.clone();

            move |output| {
                // If the channel is closed, the associated task has been cancelled
                output_sender.send(output).map_err(|_| ())?;

                waker.wake();

                // TODO: revisit the error handling in here
                Ok(())
            }
        });

        let send_request = {
            let effect = request.into();
            let effects = self.effects.clone();
            move || {
                effects
                    .send(effect)
                    .expect("Command could not send stream effect, effect channel disconnected")
            }
        };

        ShellStream::ReadyToSend(Box::new(send_request), shared_waker, output_receiver)
    }

    /// Send an event which should be handed to the update function. This is used to communicate the result
    /// (or a sequence of results) of a command back to the app so that state can be updated accordingly
    pub fn send_event(&self, event: Event) {
        self.events
            .send(event)
            .expect("Command could not send event, event channel disconnected")
    }

    /// Spawn a new task within the same command. The task will execute concurrently with other tasks within the
    /// command until it either concludes, is aborted, or until the parent command is aborted.
    ///
    /// Returns a JoinHandle which can be used as a future to await the completion of the task. It can also
    /// be used to abort the task.
    // RFC: should this have the same signature as `new` to avoid the boilerplate cloning of context in user code?
    pub fn spawn<F>(&self, future: F) -> JoinHandle
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let task = Task {
            join_handle_waker: Default::default(),
            finished: Default::default(),
            aborted: Default::default(),
            future: future.boxed(),
        };

        let handle = JoinHandle {
            join_handle_waker: task.join_handle_waker.clone(),
            finished: task.finished.clone(),
            aborted: task.aborted.clone(),
        };

        self.tasks
            .send(task)
            .expect("Command could not spawn task, tasks channel disconnected");

        handle
    }
}

pub enum ShellRequest<T: Unpin + Send> {
    ReadyToSend(Box<dyn FnOnce() + Send>, Arc<AtomicWaker>, Receiver<T>),
    Sent(Receiver<T>, Arc<AtomicWaker>),
}

impl<T: Unpin + Send> ShellRequest<T> {
    fn send(&mut self) {
        if let ShellRequest::ReadyToSend(_, atomic_waker, output_receiver) = &self {
            let ShellRequest::ReadyToSend(send_request, _, _) = std::mem::replace(
                self,
                ShellRequest::Sent(output_receiver.clone(), atomic_waker.clone()),
            ) else {
                unreachable!()
            };

            send_request()
        }
    }
}

impl<T: Unpin + Send> Future for ShellRequest<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match *self {
            ShellRequest::ReadyToSend(_, ref atomic_waker, _) => {
                atomic_waker.register(cx.waker());
                self.send();

                Poll::Pending
            }
            ShellRequest::Sent(ref receiver, ref atomic_waker) => match receiver.try_recv() {
                Ok(value) => Poll::Ready(value),
                Err(_) => {
                    atomic_waker.register(cx.waker());
                    Poll::Pending
                }
            },
        }
    }
}

pub enum ShellStream<T: Unpin + Send> {
    ReadyToSend(Box<dyn FnOnce() + Send>, Arc<AtomicWaker>, Receiver<T>),
    Sent(Receiver<T>, Arc<AtomicWaker>),
}

impl<T: Unpin + Send> ShellStream<T> {
    fn send(&mut self) {
        if let ShellStream::ReadyToSend(_, atomic_waker, output_receiver) = &self {
            let ShellStream::ReadyToSend(send_request, _, _) = std::mem::replace(
                self,
                ShellStream::Sent(output_receiver.clone(), atomic_waker.clone()),
            ) else {
                unreachable!()
            };

            send_request()
        }
    }
}

impl<T: Unpin + Send> Stream for ShellStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.deref_mut() {
            ShellStream::ReadyToSend(_, shared_waker, _) => {
                shared_waker.register(cx.waker());
                self.send();

                Poll::Pending
            }
            ShellStream::Sent(output_receiver, shared_waker) => {
                match output_receiver.try_recv() {
                    Ok(value) => Poll::Ready(Some(value)),
                    // There are no values waiting, we return pending
                    // so that the parent future knows to keep waiting
                    Err(TryRecvError::Empty) => {
                        shared_waker.register(cx.waker());
                        Poll::Pending
                    }
                    // Channel is closed, so the stream has ended
                    Err(TryRecvError::Disconnected) => Poll::Ready(None),
                }
            }
        }
    }
}
