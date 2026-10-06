#![allow(clippy::redundant_pub_crate)]
use super::super::Command;

use alloc::task::Wake;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::Ordering;
use core::task::{Context, Poll, Waker};

use crate::sync::channel::{Receiver, Sender};

use futures::future::BoxFuture;

use core::sync::atomic::AtomicBool;

use futures::task::AtomicWaker;

use alloc::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TaskId(pub(crate) usize);

// ANCHOR: task
pub(crate) struct Task {
    // Set to true when the task is aborted. Aborted tasks will poll Ready on the
    // next poll
    pub(crate) aborted: Arc<AtomicBool>,
    // Separate from the future's waker so abort handles do not keep abandoned tasks alive.
    pub(crate) abort_waker: Arc<AtomicWaker>,
    // The future polled by this task
    pub(crate) future: BoxFuture<'static, ()>,
    // Fields drop in declaration order: clean up the future before notifying joiners.
    pub(crate) completion: TaskCompletion,
}
// ANCHOR_END: task

impl Task {
    pub(crate) fn is_aborted(&self) -> bool {
        self.aborted.load(Ordering::Acquire)
    }
}

/// Reports task termination on every drop path, including command teardown and
/// tasks discarded before their first poll.
pub(crate) struct TaskCompletion {
    pub(crate) finished: Arc<AtomicBool>,
    pub(crate) join_handle_wakers: Receiver<Waker>,
}

impl Drop for TaskCompletion {
    fn drop(&mut self) {
        self.finished.store(true, Ordering::Release);
        for waker in self.join_handle_wakers.try_iter() {
            waker.wake();
        }
    }
}

// Waker provided to the tasks so they can schedule themselves to be woken
// when their future is ready to proceed.
// Waking a task also wakes the command itself, if it is being used as a Stream
// inside another Command (or hosted with a CommandSink)
// ANCHOR: command_waker
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
// ANCHOR_END: command_waker

/// A handle used to abort a Command remotely before it is complete.
///
/// Cancellation covers tasks spawned on the command and explicitly composed
/// subcommands. It is cooperative: it takes effect when the executor next
/// observes the abort request, and cannot undo work already performed.
///
/// Aborting a composed child lets its parent continue after the child has been
/// cleaned up. Aborting the enclosing command also cancels its pending `.then`
/// continuation.
///
/// Cancellation does not retract events already queued in Core or stop work
/// already handed to an external shell.
#[derive(Clone)]
pub struct AbortHandle {
    pub(crate) aborted: Arc<AtomicBool>,
    pub(crate) host_waker: Arc<AtomicWaker>,
}

impl AbortHandle {
    /// Abort the associated Command and all its tasks.
    ///
    /// Wakes the command's host so cancellation can proceed even while its tasks are suspended.
    ///
    /// Tasks must be cancellation-safe: the executor drops their futures when
    /// it observes cancellation, potentially before their first poll. A poll
    /// already in progress can finish before cancellation is observed.
    pub fn abort(&self) {
        self.aborted.store(true, Ordering::Release);
        self.host_waker.wake();
    }
}

/// A handle used to await a task completion or abort the task
#[derive(Clone)]
pub struct JoinHandle {
    pub(crate) register_waker: Sender<Waker>,
    pub(crate) finished: Arc<AtomicBool>,
    pub(crate) aborted: Arc<AtomicBool>,
    pub(crate) abort_waker: Arc<AtomicWaker>,
}

// RFC: I'm sure the ordering as used is fine...? Right? :) In all seriousness, how would
// one test this to make sure it works as intended in a multi-threaded context?
impl JoinHandle {
    /// Abort the task associated with this join handle. The task will be aborted at the
    /// next .await point. Any tasks this task spawned will continue running.
    /// Schedules the task and wakes its owning command even if the task is suspended.
    /// If the task is finished, this call does nothing.
    // RFC: Do we need to think more thoroughly about cancellation? For example, should
    // the tasks have a parent-child relationship where cancelling the parent cancels all
    // the children?
    pub fn abort(&self) {
        // The abort waker holds the slot id of the finished task. A new task can
        // reuse that slot, so a wake now would poll the wrong task.
        if self.is_finished() {
            return;
        }

        self.aborted.store(true, Ordering::Release);
        self.abort_waker.wake();
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
            // Register first, then recheck completion so termination between
            // the first check and registration cannot leave us asleep.
            if self.register_waker.send(cx.waker().clone()).is_err() || self.is_finished() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TaskState {
    Missing,
    Suspended,
    Completed,
    Cancelled,
}

// Command is actually an async executor of sorts, similar to futures::FuturesUnordered
impl<Effect, Event> Command<Effect, Event> {
    // Run all tasks until all of them are pending
    // ANCHOR: run_until_settled
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
                        drop(self.tasks.remove(task_id.0));
                    }
                }
            }
        }
    }
    // ANCHOR_END: run_until_settled

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
            let abort_waker = task.abort_waker.clone();
            let task_id = self.tasks.insert(task);

            abort_waker.register(&Waker::from(Arc::new(CommandWaker {
                task_id: TaskId(task_id),
                ready_queue: self.ready_sender.clone(),
                parent_waker: self.waker.clone(),
                woken: AtomicBool::new(false),
            })));

            // Always schedule the initial poll, including when abort happened before registration.
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
