use super::super::Command;

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::task::{Context, Poll, Wake};

use crossbeam_channel::Sender;

use futures::future::BoxFuture;

use std::sync::atomic::AtomicBool;

use futures::task::AtomicWaker;

use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub(crate) struct TaskId(pub(crate) usize);

pub(crate) struct Task {
    // Used to wake the join handle when the task concludes
    pub(crate) join_handle_waker: Arc<AtomicWaker>,
    // Set to true when the task finishes, used by the join handle
    // RFC: is there a safe way to do this relying on the waker alone?
    pub(crate) finished: Arc<AtomicBool>,
    // Set to true when the task is aborted. Aborted tasks will poll Ready on the
    // next poll
    pub(crate) aborted: Arc<AtomicBool>,
    // The future polled by this task
    pub(crate) future: BoxFuture<'static, ()>,
}

impl Task {
    pub(crate) fn is_aborted(&self) -> bool {
        self.aborted.load(Ordering::Relaxed)
    }
}

// Waker provided to the tasks so they can schedule themselves to be woken
// when their future is ready to proceed.
// Waking a task also wakes the command itself, if it is being used as a Stream
// inside another Command (or hosted with a CommandSink)
pub(crate) struct CommandWaker {
    pub(crate) task_id: TaskId,
    pub(crate) ready_queue: Sender<TaskId>,
    // Waker for the executor running this command as a Stream.
    // When the command is executed directly (e.g. in tests) this waker
    // will not be registered.
    pub(crate) parent_waker: Arc<AtomicWaker>,
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

/// A handle used to abort a Command remotely before it is complete
#[derive(Clone)]
pub struct AbortHandle {
    pub(crate) aborted: Arc<AtomicBool>,
}

impl AbortHandle {
    /// Abort the associated Command and all its tasks.
    ///
    /// The tasks will be stopped (not polled any more) at the next .await point.
    /// If you use this, make sure the tasks the Command is running are all cancellation
    /// safe, as they can be stopped at any of the await points or even before they are first polled
    pub fn abort(&self) {
        self.aborted.store(true, Ordering::Relaxed);
    }
}

/// A handle used to await a task completion of abort the task
pub struct JoinHandle {
    pub(crate) join_handle_waker: Arc<AtomicWaker>,
    pub(crate) finished: Arc<AtomicBool>,
    pub(crate) aborted: Arc<AtomicBool>,
}

// RFC: I'm sure Ordering::Relaxed is fine...? Right? :) In all seriousness, how would
// one test this to make sure it works as intended in a multi-threaded context?
impl JoinHandle {
    /// Abort the task associated with this join handle. The task will be aborted at the
    /// next .await point. Any tasks this task spawned will continue running.
    // RFC: Do we need to think more thoroughly about cancellation? For example, should
    // the tasks have a parent-child relationship where cancelling the parent cancels all
    // the children?
    pub fn abort(&self) {
        self.aborted.store(true, Ordering::Relaxed);
    }

    pub(crate) fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Relaxed)
    }
}

impl Future for JoinHandle {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.is_finished() {
            Poll::Ready(())
        } else {
            self.join_handle_waker.register(cx.waker());

            Poll::Pending
        }
    }
}

pub(crate) enum TaskState {
    Missing,
    Suspended,
    Completed,
}

// Command is actually an async executor of sorts, similar to futures::FuturesUnordered
impl<Effect, Event> Command<Effect, Event> {
    // Run all tasks until all of them are pending
    pub(crate) fn run_until_settled(&mut self) {
        if self.was_aborted() {
            self.tasks.clear();

            return;
        }

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
                        // Remove and drop the task, it's finished
                        let task = self.tasks.remove(task_id.0);

                        task.finished.store(true, Ordering::Relaxed);
                        task.join_handle_waker.wake();

                        drop(task);
                    }
                }
            }
        }
    }

    pub(crate) fn run_task(&mut self, task_id: TaskId) -> TaskState {
        let Some(task) = self.tasks.get_mut(task_id.0) else {
            return TaskState::Missing;
        };

        if task.is_aborted() {
            return TaskState::Completed;
        }

        let ready_queue = self.ready_sender.clone();
        let parent_waker = self.waker.clone();

        let waker = Arc::new(CommandWaker {
            task_id,
            ready_queue,
            parent_waker,
        })
        .into();
        let context = &mut Context::from_waker(&waker);

        match task.future.as_mut().poll(context) {
            Poll::Pending => TaskState::Suspended,
            Poll::Ready(_) => TaskState::Completed,
        }
    }

    pub(crate) fn spawn_new_tasks(&mut self) {
        while let Ok(task) = self.spawn_queue.try_recv() {
            let task_id = self.tasks.insert(task);

            self.ready_sender
                .send(TaskId(task_id))
                .expect("Command can't spawn a task, ready_queue has disconnected");
        }
    }

    pub fn was_aborted(&self) -> bool {
        self.aborted.load(Ordering::Relaxed)
    }
}
