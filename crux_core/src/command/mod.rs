use std::future::Future;
use std::ops::DerefMut as _;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use crossbeam_channel::{Receiver, Sender, TryRecvError}; // TODO: do we want to use capability channel here?
use futures::{future, Stream};
use futures::{FutureExt, StreamExt as _};
use slab::Slab;

use crate::capability::Operation;

use crate::Request;

#[derive(Clone, Copy, Debug)]
struct TaskId(usize);

type BoxFuture = future::BoxFuture<'static, ()>;

// Public API

pub struct Command<Effect, Event> {
    effects: Receiver<Effect>,
    events: Receiver<Event>,
    ready_queue: Receiver<TaskId>,
    ready_sender: Sender<TaskId>,
    tasks: Slab<BoxFuture>,
}

impl<Effect, Event> Command<Effect, Event>
where
    Effect: Send + 'static,
    Event: Send + 'static,
{
    pub fn new<F, Fut>(create_task: F) -> Self
    where
        F: FnOnce(CommandContext<Effect, Event>) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let (effect_sender, effect_receiver) = crossbeam_channel::unbounded();
        let (event_sender, event_receiver) = crossbeam_channel::unbounded();
        let (ready_sender, ready_receiver) = crossbeam_channel::unbounded();

        let context = CommandContext {
            effects: effect_sender,
            events: event_sender,
        };

        let task = create_task(context).boxed();
        let mut tasks = Slab::with_capacity(1);
        let task_id = TaskId(tasks.insert(task));

        ready_sender
            .send(task_id)
            .expect("Could not make task ready, ready channel disconnected");

        Command {
            effects: effect_receiver,
            events: event_receiver,
            ready_queue: ready_receiver,
            ready_sender,
            tasks,
        }
    }

    pub fn done() -> Self {
        let (_, effects) = crossbeam_channel::bounded(0);
        let (_, events) = crossbeam_channel::bounded(0);
        let (ready_sender, ready_queue) = crossbeam_channel::bounded(0);

        Command {
            effects,
            events,
            ready_queue,
            ready_sender,
            tasks: Slab::new(),
        }
    }

    pub fn notify_shell<Op>(operation: Op) -> Self
    where
        Op: Operation,
        Effect: From<Request<Op>>,
    {
        Command::new(|ctx| async move { ctx.notify_shell(operation) })
    }

    pub fn request_from_shell<Op, E>(operation: Op, event: E) -> Self
    where
        Op: Operation,
        Effect: From<Request<Op>>,
        E: FnOnce(Op::Output) -> Event + Send + 'static,
    {
        Command::new(|ctx| async move {
            let output = ctx.request_from_shell(operation).await;
            let event = event(output);

            ctx.send_event(event)
        })
    }

    pub fn stream_from_shell<Op, E>(operation: Op, event: E) -> Self
    where
        Op: Operation,
        Effect: From<Request<Op>>,
        E: Fn(Op::Output) -> Event + Send + 'static,
    {
        Command::new(|ctx| async move {
            let mut stream = ctx.stream_from_shell(operation);
            while let Some(output) = stream.next().await {
                ctx.send_event(event(output))
            }
        })
    }

    pub fn is_done(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Run the effect state machine until it settles and collect all effects generated
    pub fn effects(&mut self) -> Vec<Effect> {
        self.run_until_settled();

        self.effects.try_iter().collect()
    }

    /// Run the effect state machine until it settles and collect all events generated
    pub fn events(&mut self) -> Vec<Event> {
        self.run_until_settled();

        self.events.try_iter().collect()
    }
}

// Context enabling futures to communicate with the Command

#[derive(Clone)]
pub struct CommandContext<Effect, Event> {
    effects: Sender<Effect>,
    events: Sender<Event>,
}

impl<Effect, Event> CommandContext<Effect, Event> {
    fn notify_shell<Op>(&self, operation: Op)
    where
        Op: Operation,
        Effect: From<Request<Op>>,
    {
        let request = Request::resolves_never(operation);

        self.effects
            .send(request.into())
            .expect("Command could not send notification, effect channel disconnected");
    }

    fn request_from_shell<Op>(&self, operation: Op) -> ShellRequest<Op::Output>
    where
        Op: Operation,
        Effect: From<Request<Op>> + Send + 'static,
    {
        // Two way communication betwen the Request's resolve and the ShellRequest
        let (output_sender, output_receiver) = crossbeam_channel::bounded(1);
        let (waker_sender, waker_receiver) = crossbeam_channel::bounded(1);

        let request = Request::resolves_once(operation, move |output| {
            // The future sent its waker into the channel after dispatching the request.
            // We take it out in order to signal the future is ready, because the output
            // has been received and is ready to read from the output channel
            let waker: Waker = waker_receiver.try_recv().expect(
                "Shell request was resolved, but the sending future is not waiting for output",
            );

            output_sender
                .send(output)
                .expect("Request could not send output to ShellRequest future");

            waker.wake();
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

        ShellRequest::ReadyToSend(Box::new(send_request), waker_sender, output_receiver)
    }

    fn stream_from_shell<Op>(&self, operation: Op) -> ShellStream<Op::Output>
    where
        Op: Operation,
        Effect: From<Request<Op>> + Send + 'static,
    {
        // Two way communication betwen the Request's resolve and the ShellRequest
        let (output_sender, output_receiver) = crossbeam_channel::unbounded();
        let (waker_sender, waker_receiver) = crossbeam_channel::bounded(1);

        let request = Request::resolves_many_times(operation, move |output| {
            // The future sent its waker into the channel after dispatching the request.
            // We take it out in order to signal the future is ready, because the output
            // has been received and is ready to read from the output channel
            let waker: Waker = waker_receiver.try_recv().expect(
                "Shell request was resolved, but the sending future is not waiting for output",
            );

            output_sender
                .send(output)
                .expect("Request could not send output to ShellStream future");

            waker.wake();

            // TODO: revisit the error handling in here
            Ok(())
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

        ShellStream::ReadyToSend(Box::new(send_request), waker_sender, output_receiver)
    }

    fn send_event(&self, event: Event) {
        self.events
            .send(event)
            .expect("Command could not send event, event channel disconnected")
    }
}

enum ShellRequest<T: Unpin + Send> {
    ReadyToSend(Box<dyn FnOnce() + Send>, Sender<Waker>, Receiver<T>),
    Sent(Receiver<T>),
}

impl<T: Unpin + Send> Future for ShellRequest<T> {
    type Output = T;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.deref_mut() {
            ShellRequest::ReadyToSend(send_request, waker_sender, output_receiver) => {
                let waker = cx.waker().clone();

                // Need to do memory trickery in order to call the send_request
                let mut swapped_send_request: Box<dyn FnOnce() + Send + 'static> = Box::new(|| {});
                std::mem::swap(&mut swapped_send_request, send_request);

                // Prepare the waker for the resolve callback
                waker_sender
                    .send(waker)
                    .expect("ShellRequest future could not send waker to Request");

                *self = ShellRequest::Sent(output_receiver.clone());

                // Send the request
                swapped_send_request();

                Poll::Pending
            }
            ShellRequest::Sent(receiver) => {
                let value = receiver.try_recv().expect(
                    "ShellRequest future could not receive the output value from the Request",
                );

                Poll::Ready(value)
            }
        }
    }
}

enum ShellStream<T: Unpin + Send> {
    ReadyToSend(Box<dyn FnOnce() + Send>, Sender<Waker>, Receiver<T>),
    Sent(Receiver<T>, Sender<Waker>),
}

impl<T: Unpin + Send> Stream for ShellStream<T> {
    type Item = T;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.deref_mut() {
            ShellStream::ReadyToSend(send_stream_request, waker_sender, output_receiver) => {
                let waker = cx.waker().clone();

                // Need to do memory trickery in order to call the send_request
                let mut swapped_send_request: Box<dyn FnOnce() + Send + 'static> = Box::new(|| {});
                std::mem::swap(&mut swapped_send_request, send_stream_request);

                // Prepare the waker for the resolve callback
                waker_sender
                    .send(waker)
                    .expect("ShellStream future could not send waker to Request");

                *self = ShellStream::Sent(output_receiver.clone(), waker_sender.clone());

                // Send the request
                swapped_send_request();

                Poll::Pending
            }
            ShellStream::Sent(output_receiver, waker_sender) => {
                match output_receiver.try_recv() {
                    Ok(value) => {
                        // Each subsequent resolve of the Request will look for a waker in the channel,
                        // so we make sure there is one

                        let waker = cx.waker().clone();
                        waker_sender
                            .send(waker)
                            .expect("ShellStream future could not send waker to Request");

                        Poll::Ready(Some(value))
                    }
                    // There are no values waiting, we return pending
                    // so that the parent future knows to keep waiting
                    Err(TryRecvError::Empty) => Poll::Pending,
                    // Channel is closed, so the stream has ended
                    Err(TryRecvError::Disconnected) => Poll::Ready(None),
                }
            }
        }
    }
}

// Async executor stuff

struct CommandWaker {
    task_id: TaskId,
    ready_queue: Sender<TaskId>,
}

impl Wake for CommandWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        // If we can't send the id to the ready queue,
        // There is no Command to poll the task again anyway,
        // nothing to do.
        let _ = self.ready_queue.send(self.task_id);
    }
}

enum TaskState {
    Missing,
    Suspended,
    Completed,
}

// Command is actually an async executor of sorts
impl<Effect, Event> Command<Effect, Event> {
    // Run all tasks until all of them are pending
    fn run_until_settled(&mut self) {
        while let Ok(task_id) = self.ready_queue.try_recv() {
            match self.run_task(task_id) {
                TaskState::Missing => {
                    // The task has been evicted because it completed.  This can happen when
                    // a _running_ task schedules itself to wake, but then completes and gets
                    // removed
                }
                TaskState::Suspended => {
                    // Task suspended, we pick it up again when it's woken up
                }
                TaskState::Completed => {
                    // Remove and drop the task it's finished
                    drop(self.tasks.remove(task_id.0));
                }
            }
        }
    }

    fn run_task(&mut self, task_id: TaskId) -> TaskState {
        let Some(task) = self.tasks.get_mut(task_id.0) else {
            return TaskState::Missing;
        };
        let ready_queue = self.ready_sender.clone();

        let waker = Arc::new(CommandWaker {
            task_id,
            ready_queue,
        })
        .into();
        let context = &mut Context::from_waker(&waker);

        match task.as_mut().poll(context) {
            std::task::Poll::Pending => TaskState::Suspended,
            std::task::Poll::Ready(_) => TaskState::Completed,
        }
    }
}

#[cfg(test)]
mod tests;
