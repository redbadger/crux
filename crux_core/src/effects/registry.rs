use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicU32, Ordering},
    },
};

use crate::{Request, RequestHandle, ResolveError, capability::Operation};

pub struct Registry<Op: Operation> {
    next_id: AtomicU32,
    requests: Mutex<HashMap<u32, Request<Op>>>,
}

impl<Op: Operation> Default for Registry<Op> {
    fn default() -> Self {
        Self {
            next_id: AtomicU32::new(0),
            requests: Mutex::new(HashMap::new()),
        }
    }
}

impl<Op> Registry<Op>
where
    Op: Operation + Clone,
{
    /// Register an effect request for later continuing with [`Self::resolve`]. Stores
    /// The effect under an ID, returns the id and the operation the request was carrying
    ///
    /// # Panics
    ///
    /// Panics if the lock around the underlying storage was poisoned
    pub fn register(&self, request: Request<Op>) -> (u32, Op) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let operation = request.operation.clone();

        self.requests
            .lock()
            .expect("registry lock poisoned")
            .insert(id, request);

        (id, operation)
    }
}

impl<Op> Registry<Op>
where
    Op: Operation,
{
    /// Resolve an effect stored under `id` with an `output`.
    /// If the request is expected to resolve multiple times it will be reinserted
    ///
    /// # Errors
    ///
    /// Returns an Err(ResolveError) if the request cannot be resolved.
    ///
    /// # Panics
    ///
    /// Panics if the lock around the underlying storage was poisoned
    pub fn resolve(&self, id: u32, output: Op::Output) -> Result<(), ResolveError> {
        let mut requests = self.requests.lock().expect("registry lock poisoned");
        let Some(mut request) = requests.remove(&id) else {
            panic!("missing pending handle for id {id}");
        };
        drop(requests);

        request.resolve(output)?;

        if matches!(request.handle, RequestHandle::Many(_))  {
            self.requests
                .lock()
                .expect("registry lock poisoned")
                .insert(id, request);
        }

        Ok(())
    }
}
