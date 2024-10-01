use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    task::{Context, RawWaker, RawWakerVTable, Waker},
};

use crossbeam_channel::{Receiver, Sender};
use futures::{future, Future, FutureExt};
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

#[derive(Clone)]
struct NotifyTask {
    task_id: Uuid,
    sender: Sender<Uuid>,
}

impl NotifyTask {
    fn notify(&self) {
        let _ = self.sender.send(self.task_id);
    }

    fn raw_waker(self) -> RawWaker {
        let data = Box::new(self);
        let data = Box::into_raw(data) as *const _ as *const ();

        fn clone(data: *const ()) -> RawWaker {
            let data: &NotifyTask = unsafe { &*(data as *const NotifyTask) };
            data.clone().raw_waker()
        }

        fn wake(data: *const ()) {
            let data: Box<NotifyTask> =
                unsafe { Box::from_raw(data as *const NotifyTask as *mut _) };
            data.notify();
        }

        fn wake_by_ref(data: *const ()) {
            let data: &NotifyTask = unsafe { &*(data as *const NotifyTask) };
            data.notify();
        }

        fn drop(data: *const ()) {
            let _: Box<NotifyTask> = unsafe { Box::from_raw(data as *const NotifyTask as *mut _) };
        }

        RawWaker::new(data, &RawWakerVTable::new(clone, wake, wake_by_ref, drop))
    }

    fn construct_waker(self) -> Waker {
        unsafe { Waker::from_raw(self.raw_waker()) }
    }
}

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
            // While there are tasks to be processed
            while let Ok(task) = self.task_queue.try_recv() {
                let task_id = task.id();
                self.tasks.lock().unwrap().insert(task_id, task);
                self.run_task(task_id);
                did_some_work = true;
            }
            while let Ok(task_id) = self.ready_queue.try_recv() {
                self.run_task(task_id);
                did_some_work = true;
            }
        }
    }

    fn run_task(&self, task_id: Uuid) {
        let mut tasks = self.tasks.lock().unwrap();
        let mut task = tasks.remove(&task_id).unwrap();
        drop(tasks);

        let notify = task.notify();
        let waker = notify.construct_waker();
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
