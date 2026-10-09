//! The core's timers (`crux_time`'s `NotifyAfter` and `ClearTimer`), resolved by their own
//! task rather than the main loop.
//!
//! The main loop pushes each time request onto [`TIMERS`]. The [`timers`] task awaits an
//! embassy `Timer` for each `NotifyAfter` and resolves it with `Core::resolve`, which runs
//! the core forward and returns the follow-up effects. Those go onto [`EFFECTS`], which the
//! main loop selects on alongside the link and the inputs.
//!
//! The rules are the ones `crux_time`'s shipped shell handlers follow: a `NotifyAfter` is
//! answered with its own timer id when it fires, and a `ClearTimer` cancels the timer it
//! names and is answered with the same id. (This app never clears a timer, but the effect
//! is part of its `Effect` type.)

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::cell::RefCell;

use crux_core::{Core, Request};
use embassy_futures::select::select;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, Timer};

use shared::time::operation::{ClearTimer, NotifyAfter};
use shared::{Counter, Effect};

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

/// A time request from the core, for the timer task.
pub enum TimeRequest {
    NotifyAfter(Request<NotifyAfter>),
    Clear(Request<ClearTimer>),
}

/// Effects produced by the timer task, for the main loop.
pub static EFFECTS: Lane<Effect> = Lane::new();
/// Time requests, for the timer task.
pub static TIMERS: Lane<TimeRequest> = Lane::new();

/// Resolves the core's time requests with embassy timers, and hands the follow-up effects
/// to the main loop. It runs on the same executor as the main loop, so it never calls into
/// the core while the main loop is.
///
/// It awaits one `Timer`, for the earliest request, polled with this task's own waker;
/// embassy's default timer queue rejects wakers its executor did not create.
#[embassy_executor::task]
pub async fn timers(core: &'static Core<Counter>) {
    let mut pending: Vec<(Instant, Request<NotifyAfter>)> = Vec::new();
    loop {
        if let Some(due) = pending.iter().map(|(due, _)| *due).min() {
            let _ = select(TIMERS.wait(), Timer::at(due)).await;
        } else {
            TIMERS.wait().await;
        }

        for request in TIMERS.take() {
            match request {
                TimeRequest::NotifyAfter(request) => {
                    let wait = core::time::Duration::from(request.operation.duration);
                    let millis = u64::try_from(wait.as_millis()).unwrap_or(u64::MAX);
                    let due = Instant::now().saturating_add(Duration::from_millis(millis));
                    pending.push((due, request));
                }
                TimeRequest::Clear(mut request) => {
                    // Drop the timer it names, unanswered: the core has stopped listening.
                    let id = request.operation.id;
                    pending.retain(|(_, timer)| timer.operation.id != id);
                    for effect in core.resolve(&mut request, id).unwrap_or_default() {
                        EFFECTS.push(effect);
                    }
                }
            }
        }

        // ANCHOR: fire
        let now = Instant::now();
        let mut i = 0;
        while i < pending.len() {
            if pending[i].0 <= now {
                let (_, mut request) = pending.swap_remove(i);
                let id = request.operation.id;
                for effect in core.resolve(&mut request, id).unwrap_or_default() {
                    EFFECTS.push(effect);
                }
            } else {
                i += 1;
            }
        }
        // ANCHOR_END: fire
    }
}
