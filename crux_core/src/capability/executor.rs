use std::{
    sync::{Arc, Mutex},
    task::Context,
};

use crossbeam_channel::{Receiver, Sender};
use futures::{
    future,
    task::{waker_ref, ArcWake},
    Future, FutureExt,
};

// used in docs/internals/runtime.md
// ANCHOR: executor
pub(crate) struct QueuingExecutor {
    ready_queue: Receiver<Arc<Task>>,
}
// ANCHOR_END: executor

// used in docs/internals/runtime.md
// ANCHOR: spawner
#[derive(Clone)]
pub struct Spawner {
    task_sender: Sender<Arc<Task>>,
}
// ANCHOR_END: spawner

// used in docs/internals/runtime.md
// ANCHOR: task
struct Task {
    future: Mutex<Option<future::BoxFuture<'static, ()>>>,

    task_sender: Sender<Arc<Task>>,
}
// ANCHOR_END: task

pub(crate) fn executor_and_spawner() -> (QueuingExecutor, Spawner) {
    let (task_sender, ready_queue) = crossbeam_channel::unbounded();

    (QueuingExecutor { ready_queue }, Spawner { task_sender })
}

// used in docs/internals/runtime.md
// ANCHOR: spawning
impl Spawner {
    pub fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let future = future.boxed();
        let task = Arc::new(Task {
            future: Mutex::new(Some(future)),
            task_sender: self.task_sender.clone(),
        });

        self.task_sender
            .send(task)
            .expect("unable to spawn an async task, task sender channel is disconnected.")
    }
}
// ANCHOR_END: spawning

// used in docs/internals/runtime.md
// ANCHOR: arc_wake
impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let cloned = arc_self.clone();
        arc_self
            .task_sender
            .send(cloned)
            .expect("unable to wake an async task, task sender channel is disconnected.")
    }
}
// ANCHOR_END: arc_wake

// used in docs/internals/runtime.md
// ANCHOR: run_all
impl QueuingExecutor {
    pub fn run_all(&self) {
        // While there are tasks to be processed
        while let Ok(task) = self.ready_queue.try_recv() {
            // Unlock the future in the Task
            let mut future_slot = task.future.lock().unwrap();

            // Take it, replace with None, ...
            if let Some(mut future) = future_slot.take() {
                let waker = waker_ref(&task);
                let context = &mut Context::from_waker(&waker);

                // ...and poll it
                if future.as_mut().poll(context).is_pending() {
                    // If it's still pending, put it back
                    *future_slot = Some(future)
                }
            }
        }
    }
}
// ANCHOR_END: run_all

#[cfg(test)]
mod tests {
    use crate::capability::shell_request::ShellRequest;

    use super::*;

    #[test]
    fn test_task_does_not_leak() {
        let counter: Arc<()> = Arc::new(());
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
