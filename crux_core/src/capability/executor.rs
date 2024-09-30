use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    task::Context,
};

use crossbeam_channel::{Receiver, Sender};
use futures::{
    future,
    task::{waker_ref, ArcWake},
    Future, FutureExt,
};
use uuid::Uuid;

// used in docs/internals/runtime.md
// ANCHOR: executor
pub(crate) struct QueuingExecutor {
    task_queue: Receiver<Task>,
    ready_queue: Receiver<Uuid>,
    tasks: Mutex<HashMap<Uuid, Task>>,
}
// ANCHOR_END: executor

// used in docs/internals/runtime.md
// ANCHOR: spawner
#[derive(Clone)]
pub struct Spawner {
    task_sender: Sender<Task>,
    ready_sender: Sender<Uuid>,
}
// ANCHOR_END: spawner

// used in docs/internals/runtime.md
// ANCHOR: task
struct Task {
    id: Uuid,
    future: future::BoxFuture<'static, ()>,
    ready_sender: Sender<Uuid>,
}

impl Task {
    fn id(&self) -> Uuid {
        self.id
    }

    fn notify(&self) -> NotifyTask {
        NotifyTask {
            task_id: self.id,
            sender: self.ready_sender.clone(),
        }
    }
}

// ANCHOR_END: task

pub(crate) fn executor_and_spawner() -> (QueuingExecutor, Spawner) {
    let (task_sender, task_queue) = crossbeam_channel::unbounded();
    let (ready_sender, ready_queue) = crossbeam_channel::unbounded();

    (
        QueuingExecutor {
            ready_queue,
            task_queue,
            tasks: Mutex::new(HashMap::new()),
        },
        Spawner {
            task_sender,
            ready_sender,
        },
    )
}

// used in docs/internals/runtime.md
// ANCHOR: spawning
impl Spawner {
    pub fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let future = future.boxed();
        let task = Task {
            id: Uuid::new_v4(),
            future,
            ready_sender: self.ready_sender.clone(),
        };

        self.task_sender
            .send(task)
            .expect("unable to spawn an async task, task sender channel is disconnected.")
    }
}
// ANCHOR_END: spawning

// used in docs/internals/runtime.md
// ANCHOR: arc_wake
impl ArcWake for NotifyTask {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let _ = arc_self.sender.send(arc_self.task_id);
        // TODO should we report an error if send fails?
    }
}
// ANCHOR_END: arc_wake

struct NotifyTask {
    task_id: Uuid,
    sender: Sender<Uuid>,
}

// used in docs/internals/runtime.md
// ANCHOR: run_all
impl QueuingExecutor {
    pub fn run_all(&self) {
        // While there are tasks to be processed
        while let Ok(task) = self.task_queue.try_recv() {
            let task_id = task.id();
            self.tasks.lock().unwrap().insert(task_id, task);
            self.run_task(task_id);
        }
        while let Ok(task_id) = self.ready_queue.try_recv() {
            self.run_task(task_id);
        }
    }

    fn run_task(&self, task_id: Uuid) {
        let mut tasks = self.tasks.lock().unwrap();
        let mut task = tasks.remove(&task_id).unwrap();
        drop(tasks);

        let notify = Arc::new(task.notify());
        let waker = waker_ref(&notify);
        let context = &mut Context::from_waker(&waker);

        // ...and poll it
        if task.future.as_mut().poll(context).is_pending() {
            // If it's still pending, put it back
            self.tasks.lock().unwrap().insert(task.id, task);
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
