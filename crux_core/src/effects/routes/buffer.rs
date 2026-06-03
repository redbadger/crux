use std::sync::Mutex;

use crate::{Request, capability::Operation};

/// A route that simply collects requests for the caller to drain and handle.
///
/// Unlike [`Serialized`](super::Serialized) and [`Parked`](super::Parked), this
/// lane does no FFI or id bookkeeping: the routing closure pushes each request
/// with [`Buffer::push`], and the surrounding code later calls
/// [`Buffer::drain`] to take and handle them. This is convenient for tests and
/// for simple, synchronous in-process handlers.
pub struct Buffer<Op: Operation> {
    requests: Mutex<Vec<Request<Op>>>,
}

impl<Op: Operation> Default for Buffer<Op> {
    fn default() -> Self {
        Self {
            requests: Mutex::default(),
        }
    }
}

impl<Op: Operation> Buffer<Op> {
    /// Store a request for the caller to drain and handle.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex has been poisoned.
    pub fn push(&self, request: Request<Op>) {
        self.requests
            .lock()
            .expect("buffer route lock poisoned")
            .push(request);
    }

    /// Take all currently buffered requests.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex has been poisoned.
    #[must_use]
    pub fn drain(&self) -> Vec<Request<Op>> {
        self.requests
            .lock()
            .expect("buffer route lock poisoned")
            .drain(..)
            .collect()
    }
}
