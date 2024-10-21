use std::{
    sync::{Arc, Mutex},
    task::{Context, Poll, Wake},
};

use crossbeam_channel::{Receiver, Sender};
use futures::future;
use slab::Slab;

use crate::Command;

type BoxFuture<Event> = future::BoxFuture<'static, Command<Event>>;

// used in docs/internals/runtime.md
// ANCHOR: executor
pub(crate) struct QueuingExecutor<Event> {
    ready_queue: Receiver<TaskId>,
    ready_sender: Sender<TaskId>,
    tasks: Mutex<Slab<Option<BoxFuture<Event>>>>,
}

impl<Event> QueuingExecutor<Event> {
    pub(crate) fn new() -> Self {
        let (ready_sender, ready_queue) = crossbeam_channel::unbounded();
        Self {
            ready_queue,
            ready_sender,
            tasks: Mutex::new(Slab::new()),
        }
    }
}
// ANCHOR_END: executor

#[derive(Clone, Copy, Debug)]
struct TaskId(u32);

impl std::ops::Deref for TaskId {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
struct TaskWaker {
    task_id: TaskId,
    sender: Sender<TaskId>,
}

// used in docs/internals/runtime.md
// ANCHOR: wake
impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        // This send can fail if the executor has been dropped.
        // In which case, nothing to do
        let _ = self.sender.send(self.task_id);
    }
}
// ANCHOR_END: wake

// used in docs/internals/runtime.md
// ANCHOR: run_all
impl<Event> QueuingExecutor<Event> {
    pub fn spawn_task(&self, task: BoxFuture<Event>) {
        let task_id = self
            .tasks
            .lock()
            .expect("Task slab poisoned")
            .insert(Some(task));
        self.ready_sender
            .send(TaskId(task_id.try_into().expect("TaskId overflow")));
    }

    pub fn run_all(&self) -> Vec<Command<Event>> {
        let mut commands = Vec::new();
        while let Ok(task_id) = self.ready_queue.try_recv() {
            match self.run_task(task_id) {
                RunTask::Unavailable => {
                    // We were unable to run the task as it is (presumably) being run on
                    // another thread. We re-queue the task for 'later' and do NOT set
                    // `did_some_work = true`. That way we will keep looping and doing work
                    // until all remaining work is 'unavailable', at which point we will bail
                    // out of the loop, leaving the queued work to be finished by another thread.
                    // This strategy should avoid dropping work or busy-looping
                    self.ready_sender.send(task_id).expect("could not requeue");
                }
                RunTask::Missing => {
                    // This is possible if a naughty future sends a wake notification while
                    // still running, then runs to completion and is evicted from the slab.
                    // Nothing to be done.
                }
                RunTask::Suspended => {}
                RunTask::Completed(command) => commands.push(command),
            }
        }
        commands
    }

    fn run_task(&self, task_id: TaskId) -> RunTask<Event> {
        let mut lock = self.tasks.lock().expect("Task slab poisoned");
        let Some(task) = lock.get_mut(*task_id as usize) else {
            return RunTask::Missing;
        };
        let Some(mut task) = task.take() else {
            // the slot exists but the task is missing - presumably it
            // is being executed on another thread
            return RunTask::Unavailable;
        };

        // free the mutex so other threads can make progress
        drop(lock);

        let waker = Arc::new(TaskWaker {
            task_id,
            sender: self.ready_sender.clone(),
        })
        .into();
        let context = &mut Context::from_waker(&waker);

        // poll the task
        match task.as_mut().poll(context) {
            Poll::Ready(command) => {
                // otherwise the future is completed and we can free the slot
                self.tasks.lock().unwrap().remove(*task_id as usize);
                RunTask::Completed(command)
            }
            Poll::Pending => {
                // If it's still pending, put the future back in the slot
                self.tasks
                    .lock()
                    .expect("Task slab poisoned")
                    .get_mut(*task_id as usize)
                    .expect("Task slot is missing")
                    .replace(task);
                RunTask::Suspended
            }
        }
    }
}

enum RunTask<Event> {
    Missing,
    Unavailable,
    Suspended,
    Completed(Command<Event>),
}

// ANCHOR_END: run_all

#[cfg(test)]
mod tests {

    use future::FutureExt as _;
    use rand::Rng;
    use std::{
        future::Future,
        sync::atomic::{AtomicI32, Ordering},
        task::Poll,
    };

    use super::*;
    use crate::capability::shell_request::ShellRequest;

    #[test]
    fn test_task_does_not_leak() {
        // Arc is a convenient RAII counter
        let counter = Arc::new(());
        assert_eq!(Arc::strong_count(&counter), 1);

        todo!()
        // let (executor, spawner) = executor_and_spawner();

        // let future = {
        //     let counter = counter.clone();
        //     async move {
        //         assert_eq!(Arc::strong_count(&counter), 2);
        //         ShellRequest::<()>::new().await;
        //     }
        // };

        // spawner.spawn(future);
        // executor.run_all();
        // drop(executor);
        // drop(spawner);
        // assert_eq!(Arc::strong_count(&counter), 1);
    }

    #[test]
    fn test_multithreaded_executor() {
        // We define a future which chaotically sends notifications to wake up the task
        // The future has a random chance to suspend or to defer to its children which
        // may also suspend. However it will ultimately resolve to `Ready` and once it
        // has done so will stay finished
        struct Chaotic {
            ready_once: bool,
            children: Vec<Chaotic>,
        }

        static CHAOS_COUNT: AtomicI32 = AtomicI32::new(0);

        impl Chaotic {
            fn new_with_children(num_children: usize) -> Self {
                CHAOS_COUNT.fetch_add(1, Ordering::SeqCst);
                Self {
                    ready_once: false,
                    children: (0..num_children)
                        .map(|_| Chaotic::new_with_children(num_children - 1))
                        .collect(),
                }
            }
        }

        impl Future for Chaotic {
            type Output = ();

            fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                // once we're done, we're done
                if self.ready_once {
                    return Poll::Ready(());
                }
                if rand::thread_rng().gen_bool(0.1) {
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                } else {
                    let mut ready = true;
                    let this = self.get_mut();
                    for child in &mut this.children {
                        if let Poll::Pending = child.poll_unpin(cx) {
                            ready = false;
                        }
                    }
                    if ready {
                        this.ready_once = true;
                        // throw a wake in for extra chaos
                        cx.waker().wake_by_ref();
                        CHAOS_COUNT.fetch_sub(1, Ordering::SeqCst);
                        Poll::Ready(())
                    } else {
                        Poll::Pending
                    }
                }
            }
        }
        todo!()

        // let (executor, spawner) = executor_and_spawner();
        // // 100 futures with many (1957) children each equals lots of chaos
        // for _ in 0..100 {
        //     let future = Chaotic::new_with_children(6);
        //     spawner.spawn(future);
        // }
        // assert_eq!(CHAOS_COUNT.load(Ordering::SeqCst), 195700);
        // let executor = Arc::new(executor);
        // assert_eq!(executor.spawn_queue.len(), 100);

        // // Spawn 10 threads and run all
        // let handles = (0..10)
        //     .map(|_| {
        //         let executor = executor.clone();
        //         std::thread::spawn(move || {
        //             executor.run_all();
        //         })
        //     })
        //     .collect::<Vec<_>>();
        // for handle in handles {
        //     handle.join().unwrap();
        // }
        // // nothing left in queue, all futures resolved
        // assert_eq!(executor.spawn_queue.len(), 0);
        // assert_eq!(executor.ready_queue.len(), 0);
        // assert_eq!(CHAOS_COUNT.load(Ordering::SeqCst), 0);
    }
}
