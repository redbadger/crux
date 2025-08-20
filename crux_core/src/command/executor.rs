use super::super::Command;

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::task::{Context, Poll, Wake, Waker};

use crossbeam_channel::{Receiver, Sender};

use futures::future::BoxFuture;

use std::sync::atomic::AtomicBool;

use futures::task::AtomicWaker;

use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TaskId(pub(crate) usize);

pub(crate) struct Task {
    // Used to wake the join handle when the task concludes
    pub(crate) join_handle_wakers: Receiver<Waker>,
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
        self.aborted.load(Ordering::Acquire)
    }

    fn wake_join_handles(&self) {
        for waker in self.join_handle_wakers.try_iter() {
            // TODO: this potentially wakes tasks which are no longer interested
            // and wakes tasks more than once if they await multiple copies of the same join handle
            waker.wake();
        }
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
    woken: AtomicBool,
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
        self.woken.store(true, Ordering::Release);

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
        self.aborted.store(true, Ordering::Release);
    }
}

/// A handle used to await a task completion of abort the task
#[derive(Clone)]
pub struct JoinHandle {
    pub(crate) register_waker: Sender<Waker>,
    pub(crate) finished: Arc<AtomicBool>,
    pub(crate) aborted: Arc<AtomicBool>,
}

// RFC: I'm sure the ordering as used is fine...? Right? :) In all seriousness, how would
// one test this to make sure it works as intended in a multi-threaded context?
impl JoinHandle {
    /// Abort the task associated with this join handle. The task will be aborted at the
    /// next .await point. Any tasks this task spawned will continue running.
    // RFC: Do we need to think more thoroughly about cancellation? For example, should
    // the tasks have a parent-child relationship where cancelling the parent cancels all
    // the children?
    pub fn abort(&self) {
        self.aborted.store(true, Ordering::Release);
    }

    pub(crate) fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }
}

impl Future for JoinHandle {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.is_finished() {
            Poll::Ready(())
        } else {
            match self.register_waker.send(cx.waker().clone()) {
                Ok(()) => Poll::Pending,
                // The task no longer exists, we report ready immediately
                Err(_) => Poll::Ready(()),
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum TaskState {
    Missing,
    Suspended,
    Completed,
    Cancelled,
}

// Command is actually an async executor of sorts, similar to futures::FuturesUnordered
impl<Effect, Event> Command<Effect, Event> {
    // Run all tasks until all of them are pending
    pub(crate) fn run_until_settled(&mut self) {
        if self.was_aborted() {
            // Spawn new tasks to clear the spawn_queue as well
            self.spawn_new_tasks();

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
                    TaskState::Missing | TaskState::Suspended => {
                        // Missing:
                        //   The task has been evicted because it completed.  This can happen when
                        //   a _running_ task schedules itself to wake, but then completes and gets
                        //   removed
                        // Suspended:
                        //   we pick it up again when it's woken up
                    }
                    TaskState::Completed | TaskState::Cancelled => {
                        // Remove and drop the task, it's finished
                        let task = self.tasks.remove(task_id.0);

                        task.finished.store(true, Ordering::Release);
                        task.wake_join_handles();

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

        let arc_waker = Arc::new(CommandWaker {
            task_id,
            ready_queue,
            parent_waker,
            woken: AtomicBool::new(false),
        });

        let waker = arc_waker.clone().into();
        let context = &mut Context::from_waker(&waker);

        let result = match task.future.as_mut().poll(context) {
            Poll::Pending => TaskState::Suspended,
            Poll::Ready(()) => TaskState::Completed,
        };

        drop(waker);

        // If the task is pending, but there's only one copy of the waker - our one -
        // it can never be woken up again so we most likely need to evict it.
        // This happens for shell communication futures when their requests are dropped
        //
        // Note that there is an exception: the task may have used the waker and dropped it,
        // making it ready, rather than abandoned.
        let task_is_ready = arc_waker.woken.load(Ordering::Acquire);
        if result == TaskState::Suspended && !task_is_ready && Arc::strong_count(&arc_waker) < 2 {
            return TaskState::Cancelled;
        }

        result
    }

    pub(crate) fn spawn_new_tasks(&mut self) {
        while let Ok(task) = self.spawn_queue.try_recv() {
            let task_id = self.tasks.insert(task);

            self.ready_sender
                .send(TaskId(task_id))
                .expect("Command can't spawn a task, ready_queue has disconnected");
        }
    }

    #[must_use]
    pub fn was_aborted(&self) -> bool {
        self.aborted.load(Ordering::Acquire)
    }
}
