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

pub(crate) struct QueuingExecutor {
    ready_queue: Receiver<Arc<Task>>,
}

#[derive(Clone)]
pub struct Spawner {
    task_sender: Sender<Arc<Task>>,
}

struct Task {
    future: Mutex<Option<future::BoxFuture<'static, ()>>>,

    task_sender: Sender<Arc<Task>>,
}

pub(crate) fn executor_and_spawner() -> (QueuingExecutor, Spawner) {
    let (task_sender, ready_queue) = crossbeam_channel::unbounded();

    (QueuingExecutor { ready_queue }, Spawner { task_sender })
}

impl Spawner {
    pub fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let future = future.boxed();
        let task = Arc::new(Task {
            future: Mutex::new(Some(future)),
            task_sender: self.task_sender.clone(),
        });

        self.task_sender
            .send(task)
            .expect("to be able to send tasks on an unbounded queue")
    }
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let cloned = arc_self.clone();
        arc_self
            .task_sender
            .send(cloned)
            .expect("to be able to send tasks on an unbounded queue")
    }
}

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
