//! The `Delay` effect, resolved by its own task rather than the main loop.
//!
//! The main loop pushes each `Delay` request onto [`DELAYS`]. The [`delays`] task
//! awaits an embassy `Timer` for it and resolves it with `Core::resolve`, which runs
//! the core forward and returns the follow-up effects. Those go onto [`EFFECTS`],
//! which the main loop selects on alongside the link and the inputs.

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::cell::RefCell;

use crux_core::{Core, Request};
use embassy_futures::select::select;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, Timer};

use cpb_counter_http_app::{Counter, Delay, Effect};

/// An unbounded queue between two tasks on this executor, with a signal for the
/// consumer. Unbounded, so neither task can block waiting on the other.
pub struct Lane<T> {
    queue: Mutex<CriticalSectionRawMutex, RefCell<VecDeque<T>>>,
    ready: Signal<CriticalSectionRawMutex, ()>,
}

impl<T> Lane<T> {
    const fn new() -> Self {
        Self {
            queue: Mutex::new(RefCell::new(VecDeque::new())),
            ready: Signal::new(),
        }
    }

    pub fn push(&self, item: T) {
        self.queue.lock(|queue| queue.borrow_mut().push_back(item));
        self.ready.signal(());
    }

    /// Takes everything queued, and clears the signal so the consumer does not wake
    /// again for items taken here.
    pub fn take(&self) -> VecDeque<T> {
        self.ready.reset();
        self.queue
            .lock(|queue| core::mem::take(&mut *queue.borrow_mut()))
    }

    pub async fn wait(&self) {
        self.ready.wait().await;
    }
}

/// Effects produced by the delay task, for the main loop.
pub static EFFECTS: Lane<Effect> = Lane::new();
/// `Delay` requests, for the delay task.
pub static DELAYS: Lane<Request<Delay>> = Lane::new();

/// Resolves `Delay` requests with embassy timers, and hands the follow-up effects to
/// the main loop. It runs on the same executor as the main loop, so it never calls
/// into the core while the main loop is.
///
/// It awaits one `Timer`, for the earliest request, polled with this task's own
/// waker; embassy's default timer queue rejects wakers its executor did not create.
#[embassy_executor::task]
pub async fn delays(core: &'static Core<Counter>) {
    let mut pending: Vec<(Instant, Request<Delay>)> = Vec::new();
    loop {
        if let Some(due) = pending.iter().map(|(due, _)| *due).min() {
            let _ = select(DELAYS.wait(), Timer::at(due)).await;
        } else {
            DELAYS.wait().await;
        }

        for request in DELAYS.take() {
            let due = Instant::now() + Duration::from_millis(request.operation.millis.into());
            pending.push((due, request));
        }
        let now = Instant::now();
        let mut i = 0;
        while i < pending.len() {
            if pending[i].0 <= now {
                let (_, mut request) = pending.swap_remove(i);
                for effect in core.resolve(&mut request, ()).unwrap_or_default() {
                    EFFECTS.push(effect);
                }
            } else {
                i += 1;
            }
        }
    }
}
