use std::sync::Mutex;

use crate::{Request, capability::Operation};

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
