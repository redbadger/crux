//! `spike(no_std)`: internal synchronisation shim.
//!
//! Two kinds of lock:
//!
//! - `Mutex`, for the short-held internal locks (the channel queue and the
//!   effect registries). Closure-based in both builds: `std::sync::Mutex`
//!   under `std` (panicking on poison), and a `critical_section::Mutex`
//!   otherwise. A critical section masks interrupts (and, on multi-core chips,
//!   takes a hardware spinlock), so a waker firing from an interrupt handler
//!   can never find the lock held by the code it interrupted. Keep the closure
//!   short, and keep wakers and allocation out of it where possible.
//! - `CoreMutex` and `CoreRwLock`, for `Core`'s model and root command,
//!   which are held across `App::update` and the whole process loop. Under
//!   `std` they block, as before. Without it they are try-locks that panic on
//!   contention, so interrupts stay enabled during an update and "a `Core` is
//!   used from one execution context at a time" is a checked rule.
//!
//! Channel: a minimal unbounded MPSC queue replacing both `crossbeam-channel`
//! and `futures::channel::mpsc`, used in both std and no_std builds.

#![allow(clippy::redundant_pub_crate)]

#[cfg(not(any(feature = "std", feature = "critical-section")))]
compile_error!(
    "crux_core needs either the `std` feature or, without std, the `critical-section` \
     feature (and a critical-section implementation linked into the final binary)"
);

#[cfg(feature = "std")]
mod locks {
    pub(crate) struct Mutex<T>(std::sync::Mutex<T>);

    impl<T> Mutex<T> {
        pub(crate) const fn new(value: T) -> Self {
            Self(std::sync::Mutex::new(value))
        }

        pub(crate) fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
            f(&mut self.0.lock().expect("crux_core: Mutex poisoned"))
        }
    }

    impl<T: Default> Default for Mutex<T> {
        fn default() -> Self {
            Self::new(T::default())
        }
    }

    pub(crate) struct CoreMutex<T>(std::sync::Mutex<T>);

    impl<T> CoreMutex<T> {
        pub(crate) const fn new(value: T) -> Self {
            Self(std::sync::Mutex::new(value))
        }

        pub(crate) fn lock(&self) -> std::sync::MutexGuard<'_, T> {
            self.0.lock().expect("crux_core: Mutex poisoned")
        }
    }

    pub(crate) struct CoreRwLock<T>(std::sync::RwLock<T>);

    impl<T> CoreRwLock<T> {
        pub(crate) const fn new(value: T) -> Self {
            Self(std::sync::RwLock::new(value))
        }

        pub(crate) fn read(&self) -> std::sync::RwLockReadGuard<'_, T> {
            self.0.read().expect("crux_core: RwLock poisoned")
        }

        pub(crate) fn write(&self) -> std::sync::RwLockWriteGuard<'_, T> {
            self.0.write().expect("crux_core: RwLock poisoned")
        }
    }

    impl<T: Default> Default for CoreRwLock<T> {
        fn default() -> Self {
            Self::new(T::default())
        }
    }
}

#[cfg(all(not(feature = "std"), feature = "critical-section"))]
mod locks {
    use core::cell::RefCell;

    pub(crate) struct Mutex<T>(critical_section::Mutex<RefCell<T>>);

    impl<T> Mutex<T> {
        pub(crate) const fn new(value: T) -> Self {
            Self(critical_section::Mutex::new(RefCell::new(value)))
        }

        /// Runs `f` inside a critical section. Re-entering the same lock from
        /// `f` panics (the `RefCell` is already borrowed) rather than deadlocking.
        pub(crate) fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
            critical_section::with(|cs| f(&mut self.0.borrow_ref_mut(cs)))
        }
    }

    impl<T: Default> Default for Mutex<T> {
        fn default() -> Self {
            Self::new(T::default())
        }
    }

    const CONTENDED: &str = "crux_core: Core used from two execution contexts at once \
         (without std, a Core must only be used from one thread, task or core, \
         and never from an interrupt handler)";

    /// Exclusive try-lock: there is no waiting without `std`.
    pub(crate) struct CoreMutex<T>(try_lock::TryLock<T>);

    impl<T> CoreMutex<T> {
        pub(crate) const fn new(value: T) -> Self {
            Self(try_lock::TryLock::new(value))
        }

        pub(crate) fn lock(&self) -> try_lock::Locked<'_, T> {
            self.0.try_lock().expect(CONTENDED)
        }
    }

    /// Without `std` reads are exclusive too: with one owning context there
    /// is no reader concurrency to lose.
    pub(crate) struct CoreRwLock<T>(CoreMutex<T>);

    impl<T> CoreRwLock<T> {
        pub(crate) const fn new(value: T) -> Self {
            Self(CoreMutex::new(value))
        }

        pub(crate) fn read(&self) -> try_lock::Locked<'_, T> {
            self.0.lock()
        }

        pub(crate) fn write(&self) -> try_lock::Locked<'_, T> {
            self.0.lock()
        }
    }

    impl<T: Default> Default for CoreRwLock<T> {
        fn default() -> Self {
            Self::new(T::default())
        }
    }
}

pub(crate) use locks::{CoreMutex, CoreRwLock, Mutex};

pub mod channel {
    //! Unbounded multi-producer, single-consumer channel.
    //!
    //! Disconnect semantics matter to the Command executor and are kept the
    //! same as crossbeam / futures mpsc:
    //! - `send` fails (returning the value) once the receiver is dropped.
    //! - dropping the receiver drops everything still queued (so wakers and
    //!   resolve callbacks inside are released).
    //! - dropping the last sender wakes the async receiver, which then yields
    //!   `None` once the queue is drained, without registering a waker again.

    use alloc::collections::VecDeque;
    use alloc::sync::Arc;
    use core::pin::Pin;
    use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use core::task::{Context, Poll};

    use futures::Stream;
    use futures::task::AtomicWaker;

    use super::Mutex;

    struct Inner<T> {
        queue: Mutex<VecDeque<T>>,
        senders: AtomicUsize,
        receiver_alive: AtomicBool,
        waker: AtomicWaker,
    }

    pub(crate) struct Sender<T> {
        inner: Arc<Inner<T>>,
    }

    pub struct Receiver<T> {
        inner: Arc<Inner<T>>,
    }

    pub(crate) struct SendError<T>(pub(crate) T);

    // Manual impl, so `.expect()` works without `T: Debug` (as crossbeam's does).
    impl<T> core::fmt::Debug for SendError<T> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("SendError(..)")
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    pub(crate) enum TryRecvError {
        Empty,
        Disconnected,
    }

    pub(crate) fn unbounded<T>() -> (Sender<T>, Receiver<T>) {
        let inner = Arc::new(Inner {
            queue: Mutex::new(VecDeque::new()),
            senders: AtomicUsize::new(1),
            receiver_alive: AtomicBool::new(true),
            waker: AtomicWaker::new(),
        });
        (
            Sender {
                inner: inner.clone(),
            },
            Receiver { inner },
        )
    }

    impl<T> Sender<T> {
        pub(crate) fn send(&self, value: T) -> Result<(), SendError<T>> {
            if !self.inner.receiver_alive.load(Ordering::Acquire) {
                return Err(SendError(value));
            }
            self.inner.queue.with(|queue| {
                // Re-check under the lock, so we never queue after the receiver
                // has cleared the queue on drop.
                if self.inner.receiver_alive.load(Ordering::Acquire) {
                    queue.push_back(value);
                    Ok(())
                } else {
                    Err(SendError(value))
                }
            })?;
            // Wake outside the lock.
            self.inner.waker.wake();
            Ok(())
        }
    }

    impl<T> Clone for Sender<T> {
        fn clone(&self) -> Self {
            self.inner.senders.fetch_add(1, Ordering::AcqRel);
            Self {
                inner: self.inner.clone(),
            }
        }
    }

    impl<T> Drop for Sender<T> {
        fn drop(&mut self) {
            if self.inner.senders.fetch_sub(1, Ordering::AcqRel) == 1 {
                // Last sender: wake (and so release) the receiver's waker.
                self.inner.waker.wake();
            }
        }
    }

    impl<T> Receiver<T> {
        pub(crate) fn try_recv(&self) -> Result<T, TryRecvError> {
            let popped = self.inner.queue.with(VecDeque::pop_front);
            popped.ok_or_else(|| {
                if self.inner.senders.load(Ordering::Acquire) == 0 {
                    TryRecvError::Disconnected
                } else {
                    TryRecvError::Empty
                }
            })
        }

        pub(crate) fn try_iter(&self) -> impl Iterator<Item = T> + '_ {
            core::iter::from_fn(|| self.try_recv().ok())
        }

        pub(crate) fn is_empty(&self) -> bool {
            self.inner.queue.with(|queue| queue.is_empty())
        }
    }

    impl<T> Drop for Receiver<T> {
        fn drop(&mut self) {
            self.inner.receiver_alive.store(false, Ordering::Release);
            // Drop queued values outside the lock: their destructors may touch
            // other channels.
            let drained = self.inner.queue.with(core::mem::take);
            drop(drained);
        }
    }

    impl<T> Stream for Receiver<T> {
        type Item = T;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
            match self.try_recv() {
                Ok(value) => return Poll::Ready(Some(value)),
                Err(TryRecvError::Disconnected) => return Poll::Ready(None),
                Err(TryRecvError::Empty) => {}
            }

            self.inner.waker.register(cx.waker());

            // Re-check after registering, to avoid a lost wake-up.
            match self.try_recv() {
                Ok(value) => Poll::Ready(Some(value)),
                Err(TryRecvError::Disconnected) => {
                    // Release the waker we just registered
                    drop(self.inner.waker.take());
                    Poll::Ready(None)
                }
                Err(TryRecvError::Empty) => Poll::Pending,
            }
        }
    }
}
