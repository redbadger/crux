use std::{
    sync::{Arc, Mutex},
    task::{Context, Wake},
};

use crossbeam_channel::{Receiver, Sender};
use futures::{future, Future, FutureExt};
use slab::Slab;

type BoxFuture = future::BoxFuture<'static, ()>;

// used in docs/internals/runtime.md
// ANCHOR: executor
pub(crate) struct QueuingExecutor {
    spawn_queue: Receiver<BoxFuture>,
    ready_queue: Receiver<TaskId>,
    ready_sender: Sender<TaskId>,
    tasks: Mutex<Slab<Option<BoxFuture>>>,
}
// ANCHOR_END: executor

// used in docs/internals/runtime.md
// ANCHOR: spawner
#[derive(Clone)]
pub struct Spawner {
    future_sender: Sender<BoxFuture>,
}
// ANCHOR_END: spawner

#[derive(Clone, Copy, Debug)]
struct TaskId(u32);

impl std::ops::Deref for TaskId {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(crate) fn executor_and_spawner() -> (QueuingExecutor, Spawner) {
    let (future_sender, spawn_queue) = crossbeam_channel::unbounded();
    let (ready_sender, ready_queue) = crossbeam_channel::unbounded();

    (
        QueuingExecutor {
            ready_queue,
            spawn_queue,
            ready_sender,
            tasks: Mutex::new(Slab::new()),
        },
        Spawner { future_sender },
    )
}

// used in docs/internals/runtime.md
// ANCHOR: spawning
impl Spawner {
    pub fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let future = future.boxed();
        self.future_sender
            .send(future)
            .expect("unable to spawn an async task, task sender channel is disconnected.")
    }
}
// ANCHOR_END: spawning

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
impl QueuingExecutor {
    pub fn run_all(&self) {
        // we read off both queues and execute the tasks we receive.
        // Since either queue can generate work for the other queue,
        // we read from them in a loop until we are sure both queues
        // are exhaused
        let mut did_some_work = true;

        while did_some_work {
            did_some_work = false;
            while let Ok(task) = self.spawn_queue.try_recv() {
                let task_id = self.tasks.lock().unwrap().insert(Some(task));
                self.run_task(TaskId(task_id.try_into().expect("TaskId overflow")));
                did_some_work = true;
            }
            while let Ok(task_id) = self.ready_queue.try_recv() {
                self.run_task(task_id);
                did_some_work = true;
            }
        }
    }

    fn run_task(&self, task_id: TaskId) {
        let mut tasks = self.tasks.lock().unwrap();
        let mut task = tasks
            .get_mut(*task_id as usize)
            .and_then(|task| task.take())
            .expect("Task is missing");

        // free the lock
        drop(tasks);

        let waker = Arc::new(TaskWaker {
            task_id,
            sender: self.ready_sender.clone(),
        })
        .into();
        let context = &mut Context::from_waker(&waker);

        // ...and poll it
        if task.as_mut().poll(context).is_pending() {
            // If it's still pending, put the future back in the slot
            *self
                .tasks
                .lock()
                .unwrap()
                .get_mut(*task_id as usize)
                .expect("Task slot is misisng") = Some(task);
        } else {
            // otherwise the future is completed and we can free the slot
            self.tasks.lock().unwrap().remove(*task_id as usize);
        }
    }
}
// ANCHOR_END: run_all

#[cfg(test)]
mod tests {

    use super::*;
    use crate::capability::shell_request::ShellRequest;

    #[test]
    fn test_task_does_not_leak() {
        // Arc is a convenient RAII counter
        let counter = Arc::new(());
        assert_eq!(Arc::strong_count(&counter), 1);

        let (executor, spawner) = executor_and_spawner();

        let future = {
            let counter = counter.clone();
            async move {
                assert_eq!(Arc::strong_count(&counter), 2);
                ShellRequest::<()>::new().await;
            }
        };

        spawner.spawn(future);
        executor.run_all();
        drop(executor);
        drop(spawner);
        assert_eq!(Arc::strong_count(&counter), 1);
    }
}
