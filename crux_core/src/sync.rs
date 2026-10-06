//! `spike(no_std)`: internal synchronisation shim.
//!
//! Locks: `std::sync` under `std` (panicking on poison, as the call sites used
//! to), `spin` otherwise. Both expose `lock()` / `read()` / `write()` returning
//! the guard directly.
//!
//! Channel: a minimal unbounded MPSC queue replacing both `crossbeam-channel`
//! and `futures::channel::mpsc`, used in both std and no_std builds.

#![allow(clippy::redundant_pub_crate)]

#[cfg(feature = "std")]
mod locks {
    pub(crate) struct Mutex<T: ?Sized>(std::sync::Mutex<T>);

    impl<T> Mutex<T> {
        pub(crate) const fn new(value: T) -> Self {
            Self(std::sync::Mutex::new(value))
        }
    }

    impl<T: Default> Default for Mutex<T> {
        fn default() -> Self {
            Self::new(T::default())
        }
    }

    impl<T: ?Sized> Mutex<T> {
        pub(crate) fn lock(&self) -> std::sync::MutexGuard<'_, T> {
            self.0.lock().expect("crux_core: Mutex poisoned")
        }
    }

    pub(crate) struct RwLock<T: ?Sized>(std::sync::RwLock<T>);

    impl<T> RwLock<T> {
        pub(crate) const fn new(value: T) -> Self {
            Self(std::sync::RwLock::new(value))
        }
    }

    impl<T: Default> Default for RwLock<T> {
        fn default() -> Self {
            Self::new(T::default())
        }
    }

    impl<T: ?Sized> RwLock<T> {
        pub(crate) fn read(&self) -> std::sync::RwLockReadGuard<'_, T> {
            self.0.read().expect("crux_core: RwLock poisoned")
        }

        pub(crate) fn write(&self) -> std::sync::RwLockWriteGuard<'_, T> {
            self.0.write().expect("crux_core: RwLock poisoned")
        }
    }
}

#[cfg(not(feature = "std"))]
mod locks {
    pub(crate) use spin::{Mutex, RwLock};
}

pub(crate) use locks::{Mutex, RwLock};

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
            {
                let mut queue = self.inner.queue.lock();
                // Re-check under the lock, so we never queue after the receiver
                // has cleared the queue on drop.
                if !self.inner.receiver_alive.load(Ordering::Acquire) {
                    return Err(SendError(value));
                }
                queue.push_back(value);
            }
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
            let popped = self.inner.queue.lock().pop_front();
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
            self.inner.queue.lock().is_empty()
        }
    }

    impl<T> Drop for Receiver<T> {
        fn drop(&mut self) {
            self.inner.receiver_alive.store(false, Ordering::Release);
            // Drop queued values outside the lock: their destructors may touch
            // other channels.
            let drained = core::mem::take(&mut *self.inner.queue.lock());
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
