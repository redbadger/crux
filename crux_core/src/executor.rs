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

pub(crate) struct Executor {
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

pub(crate) fn executor_and_spawner() -> (Executor, Spawner) {
    let (task_sender, ready_queue) = crossbeam_channel::unbounded();

    (Executor { ready_queue }, Spawner { task_sender })
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

impl Executor {
    pub fn run_all(&self) {
        while let Ok(task) = self.ready_queue.try_recv() {
            let mut future_slot = task.future.lock().unwrap();
            if let Some(mut future) = future_slot.take() {
                let waker = waker_ref(&task);
                let context = &mut Context::from_waker(&waker);
                if future.as_mut().poll(context).is_pending() {
                    *future_slot = Some(future)
                }
            }
        }
    }
}
