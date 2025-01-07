mod builder;

use std::future::Future;
use std::ops::DerefMut as _;
use std::sync::Arc;
use std::task::{Context, Poll, Wake};

use crossbeam_channel::{Receiver, Sender, TryRecvError};
use futures::future::BoxFuture;
use futures::task::AtomicWaker;
// TODO: do we want to use capability channel here?
use futures::{stream, Stream};
use futures::{FutureExt, StreamExt as _};
use slab::Slab;

use crate::capability::Operation;

use crate::Request;

#[derive(Clone, Copy, Debug)]
struct TaskId(usize);

type CommandTask = BoxFuture<'static, ()>;

pub struct Command<Effect, Event> {
    effects: Receiver<Effect>,
    events: Receiver<Event>,
    ready_queue: Receiver<TaskId>,
    spawn_queue: Receiver<CommandTask>,
    tasks: Slab<CommandTask>,
    ready_sender: Sender<TaskId>, // Used to make wakers
    waker: Arc<AtomicWaker>,      // Shared with task wakers when polled in async context
}

// Public API

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
        // RFC: do we need to think about backpressure? The channels are unbounded
        // so a naughty Command can make massive amounts of requests or spawn a huge number of tasks.
        // If these channels supported async, the CommandContext methods could also be async and
        // we could give the channels some bounds
        let (effect_sender, effect_receiver) = crossbeam_channel::unbounded();
        let (event_sender, event_receiver) = crossbeam_channel::unbounded();
        let (ready_sender, ready_receiver) = crossbeam_channel::unbounded();
        let (spawn_sender, spawn_receiver) = crossbeam_channel::unbounded();

        let context = CommandContext {
            effects: effect_sender,
            events: event_sender,
            tasks: spawn_sender,
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
            spawn_queue: spawn_receiver,
            ready_sender,
            tasks,
            waker: Default::default(),
        }
    }

    pub fn done() -> Self {
        let (_, effects) = crossbeam_channel::bounded(0);
        let (_, events) = crossbeam_channel::bounded(0);
        let (_, spawn_queue) = crossbeam_channel::bounded(0);
        let (ready_sender, ready_queue) = crossbeam_channel::bounded(0);

        Command {
            effects,
            events,
            ready_queue,
            spawn_queue,
            tasks: Slab::with_capacity(0),
            ready_sender,
            waker: Default::default(),
        }
    }

    pub fn event(event: Event) -> Self {
        Command::new(|ctx| async move { ctx.send_event(event) })
    }

    pub fn notify_shell<Op>(operation: Op) -> Command<Effect, Event>
    where
        Op: Operation,
        Effect: From<Request<Op>>,
    {
        Command::new(|ctx| async move { ctx.notify_shell(operation) })
    }

    pub fn request_from_shell<Op>(
        operation: Op,
    ) -> builder::RequestBuilder<Effect, Event, impl Future<Output = Op::Output>>
    where
        Op: Operation,
        Effect: From<Request<Op>>,
    {
        builder::RequestBuilder::new(|ctx| ctx.request_from_shell(operation))
    }

    pub fn stream_from_shell<Op>(
        operation: Op,
    ) -> builder::StreamBuilder<Effect, Event, impl Stream<Item = Op::Output>>
    where
        Op: Operation,
        Effect: From<Request<Op>>,
    {
        builder::StreamBuilder::new(|ctx| ctx.stream_from_shell(operation))
    }

    pub fn is_done(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Run the effect state machine until it settles and collect all effects generated
    // RFC: should this collect?
    pub fn effects(&mut self) -> Vec<Effect> {
        self.run_until_settled();

        self.effects.try_iter().collect()
    }

    /// Run the effect state machine until it settles and collect all events generated
    // RFC: should this collect?
    pub fn events(&mut self) -> Vec<Event> {
        self.run_until_settled();

        self.events.try_iter().collect()
    }

    // Combinators

    /// Create a command running self and the other command in sequence
    // RFC: is this actually _useful_? Unlike `.and_then` on `CommandBuilder` this doesn't allow using
    // the output of the first command in building the second one
    pub fn then(mut self, mut other: Self) -> Self
    where
        Effect: Unpin,
        Event: Unpin,
    {
        Command::new(|ctx| async move {
            // first run self until done
            while let Some(output) = self.next().await {
                match output {
                    CommandOutput::Effect(effect) => ctx
                        .effects
                        .send(effect)
                        .expect("Cannot send effect from then future"),
                    CommandOutput::Event(event) => ctx
                        .events
                        .send(event)
                        .expect("Cannot send event from then future"),
                }
            }

            // then run other until done
            while let Some(output) = other.next().await {
                match output {
                    CommandOutput::Effect(effect) => ctx
                        .effects
                        .send(effect)
                        .expect("Cannot send effect from then future"),
                    CommandOutput::Event(event) => ctx
                        .events
                        .send(event)
                        .expect("Cannot send event from then future"),
                }
            }
        })
    }

    /// Convenience for `Command:all` which runs another command concurrently with this one
    pub fn and(self, other: Self) -> Self
    where
        Effect: Unpin,
        Event: Unpin,
    {
        Command::all([self, other])
    }

    /// Create a command running a number of commands concurrently
    pub fn all<I>(commands: I) -> Self
    where
        I: IntoIterator<Item = Self> + Send + 'static,
        Effect: Unpin,
        Event: Unpin,
    {
        Command::new(|ctx| async move {
            let mut select = stream::select_all(commands);

            while let Some(output) = select.next().await {
                match output {
                    CommandOutput::Effect(effect) => ctx
                        .effects
                        .send(effect)
                        .expect("Cannot send effect from all future"),
                    CommandOutput::Event(event) => ctx
                        .events
                        .send(event)
                        .expect("Cannot send event from all future"),
                }
            }
        })
    }

    // Mapping for composition

    pub fn map_effect<F, NewEffect>(self, map: F) -> Command<NewEffect, Event>
    where
        F: Fn(Effect) -> NewEffect + Send + 'static,
        NewEffect: Send + 'static,
        Effect: Unpin,
        Event: Unpin,
    {
        Command::new(|ctx| async move {
            let mut stream = self;

            while let Some(output) = stream.next().await {
                match output {
                    CommandOutput::Effect(effect) => ctx
                        .effects
                        .send(map(effect))
                        .expect("Cannot send effect from map_effect future"),
                    CommandOutput::Event(event) => ctx
                        .events
                        .send(event)
                        .expect("Cannot send event from map_effect future"),
                }
            }
        })
    }

    pub fn map_event<F, NewEvent>(self, map: F) -> Command<Effect, NewEvent>
    where
        F: Fn(Event) -> NewEvent + Send + 'static,
        NewEvent: Send + 'static,
        Effect: Unpin,
        Event: Unpin,
    {
        Command::new(|ctx| async move {
            let mut stream = self;

            while let Some(output) = stream.next().await {
                match output {
                    CommandOutput::Effect(effect) => ctx
                        .effects
                        .send(effect)
                        .expect("Cannot send effect from map_event future"),
                    CommandOutput::Event(event) => ctx
                        .events
                        .send(map(event))
                        .expect("Cannot send event from map_event future"),
                }
            }
        })
    }
}

// Context enabling futures to communicate with the Command
pub struct CommandContext<Effect, Event> {
    effects: Sender<Effect>,
    events: Sender<Event>,
    tasks: Sender<CommandTask>,
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

    pub fn send_event(&self, event: Event) {
        self.events
            .send(event)
            .expect("Command could not send event, event channel disconnected")
    }

    // RFC: this could return a join handle á la tokio, used to either await completion of the command or to cancel it early
    // RFC: should this have the same signature as `new` to avoid the boilerplate cloning of context in user code?
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.tasks
            .send(future.boxed())
            .expect("Command could not spawn task, tasks channel disconnected")
    }
}

pub enum ShellRequest<T: Unpin + Send> {
    ReadyToSend(Box<dyn FnOnce() + Send>, Arc<AtomicWaker>, Receiver<T>),
    Sent(Receiver<T>, Arc<AtomicWaker>),
}

impl<T: Unpin + Send> Future for ShellRequest<T> {
    type Output = T;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.deref_mut() {
            ShellRequest::ReadyToSend(send_request, atomic_waker, output_receiver) => {
                // Need to do memory trickery in order to call the send_request
                let mut swapped_send_request: Box<dyn FnOnce() + Send + 'static> = Box::new(|| {});
                std::mem::swap(&mut swapped_send_request, send_request);

                // Prepare the waker for the resolve callback
                atomic_waker.register(cx.waker());

                *self = ShellRequest::Sent(output_receiver.clone(), atomic_waker.clone());

                // Send the request
                swapped_send_request();

                Poll::Pending
            }
            ShellRequest::Sent(receiver, atomic_waker) => match receiver.try_recv() {
                Ok(value) => Poll::Ready(value),
                // not ready yet. We may be polled in a join for example
                // TODO: do we need to send the waker again here? It has not changed
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

impl<T: Unpin + Send> Stream for ShellStream<T> {
    type Item = T;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.deref_mut() {
            ShellStream::ReadyToSend(send_stream_request, shared_waker, output_receiver) => {
                shared_waker.register(cx.waker());

                // Need to do memory trickery in order to call the send_request
                let mut swapped_send_request: Box<dyn FnOnce() + Send + 'static> = Box::new(|| {});
                std::mem::swap(&mut swapped_send_request, send_stream_request);

                *self = ShellStream::Sent(output_receiver.clone(), shared_waker.clone());

                // Send the request
                swapped_send_request();

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

// Async executor stuff

struct CommandWaker {
    task_id: TaskId,
    ready_queue: Sender<TaskId>,
    parent_waker: Arc<AtomicWaker>,
}

impl Wake for CommandWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        // If we can't send the id to the ready queue, there is no Command to poll the task again anyway,
        // nothing to do.
        // TODO: Does that mean we should bail, since waking ourselves is
        // now pointless?
        let _ = self.ready_queue.send(self.task_id);

        // Note: calling `wake` before `register` is a no-op
        self.parent_waker.wake();
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
        loop {
            self.spawn_new_tasks();

            if self.ready_queue.is_empty() {
                break;
            }

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
    }

    // Run task within our _own_ context
    fn run_task(&mut self, task_id: TaskId) -> TaskState {
        let Some(task) = self.tasks.get_mut(task_id.0) else {
            return TaskState::Missing;
        };
        let ready_queue = self.ready_sender.clone();
        let parent_waker = self.waker.clone();

        let waker = Arc::new(CommandWaker {
            task_id,
            ready_queue,
            parent_waker,
        })
        .into();
        let context = &mut Context::from_waker(&waker);

        match task.as_mut().poll(context) {
            std::task::Poll::Pending => TaskState::Suspended,
            std::task::Poll::Ready(_) => TaskState::Completed,
        }
    }

    fn spawn_new_tasks(&mut self) {
        while let Ok(task) = self.spawn_queue.try_recv() {
            let task_id = self.tasks.insert(task);

            self.ready_sender
                .send(TaskId(task_id))
                .expect("Command can't spawn a task, ready_queue has disconnected");
        }
    }
}

// Command is an async Stream

#[derive(Debug)]
pub enum CommandOutput<Effect, Event> {
    Effect(Effect),
    Event(Event),
}

impl<Effect, Event> Stream for Command<Effect, Event>
where
    Effect: Unpin + Send + 'static,
    Event: Unpin + Send + 'static,
{
    type Item = CommandOutput<Effect, Event>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.waker.register(cx.waker());

        // run_until_settled is idempotent
        self.deref_mut().run_until_settled();

        // Check events first to preserve the order in which items were emitted. This is because
        // sending events doesn't yield, and the next request/stream await point will be
        // reached in the same poll, so any follow up effects will _also_ be available
        if let Ok(event) = self.events.try_recv() {
            return Poll::Ready(Some(CommandOutput::Event(event)));
        }

        if let Ok(effect) = self.effects.try_recv() {
            return Poll::Ready(Some(CommandOutput::Effect(effect)));
        };

        if self.is_done() {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests;
