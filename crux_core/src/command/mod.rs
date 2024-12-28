use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Wake};

use crossbeam_channel::{Receiver, Sender}; // TODO: do we want to use capability channel here?
use futures::future;
use futures::task::ArcWake;
use futures::FutureExt;
use slab::Slab;

use crate::capability::Operation;
use crate::Request;

#[derive(Clone, Copy, Debug)]
struct TaskId(usize);

type BoxFuture = future::BoxFuture<'static, ()>;

pub struct Command<Effect> {
    effects: Receiver<Effect>,
    ready_queue: Receiver<TaskId>,
    tasks: Slab<BoxFuture>,
}

pub struct CommandContext<Effect> {
    effects: Sender<Effect>,
}

impl<Effect> CommandContext<Effect> {
    fn notify_shell<Op>(&self, operation: Op)
    where
        Op: Operation,
        Effect: From<Request<Op>>,
    {
        let request = Request::resolves_never(operation);

        self.effects
            .send(request.into())
            .expect("Could not send notification, effect channel disconnected");
    }
}

impl<Effect> Command<Effect>
where
    Effect: Send + 'static,
{
    pub fn new<F, Fut>(create_task: F) -> Self
    where
        F: FnOnce(CommandContext<Effect>) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let (effect_sender, effect_receiver) = crossbeam_channel::unbounded();
        let (ready_sender, ready_receiver) = crossbeam_channel::unbounded();

        let context = CommandContext {
            effects: effect_sender,
        };

        let task = create_task(context).boxed();
        let mut tasks = Slab::with_capacity(1);
        let task_id = TaskId(tasks.insert(task));

        ready_sender
            .send(task_id)
            .expect("Could not make task ready, ready channel disconnected");

        Command {
            effects: effect_receiver,
            ready_queue: ready_receiver,
            tasks,
        }
    }

    pub fn done() -> Self {
        let (_, effects) = crossbeam_channel::bounded(0);
        let (_, ready_queue) = crossbeam_channel::bounded(0);

        Command {
            effects,
            ready_queue,
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

    pub fn is_done(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn effects(&mut self) -> Vec<Effect> {
        self.run_until_settled();

        self.effects.try_iter().collect()
    }
}

struct CommandWaker {
    task_id: TaskId,
}

impl Wake for CommandWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        // TODO: actually notify the command,
        // otherwise we're stuck on the first .await point
    }
}

enum TaskState {
    Missing,
    Suspended,
    Completed,
}

// Command is actually an async executor of sorts
impl<Effect> Command<Effect> {
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

        let waker = Arc::new(CommandWaker { task_id }).into();
        let context = &mut Context::from_waker(&waker);

        match task.as_mut().poll(context) {
            std::task::Poll::Pending => TaskState::Suspended,
            std::task::Poll::Ready(_) => TaskState::Completed,
        }
    }
}

#[cfg(test)]
mod tests;
