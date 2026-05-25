mod effect_id;
mod storage;

use std::sync::Mutex;

use crate::{Request, RequestHandle, ResolveError, capability::Operation};
pub use effect_id::EffectId;

pub struct Registry<Op: Operation> {
    requests: Mutex<storage::Storage<Op::Output>>,
}

impl<Op: Operation> Default for Registry<Op> {
    fn default() -> Self {
        Self {
            requests: Mutex::new(storage::Storage::default()),
        }
    }
}

impl<Op> Registry<Op>
where
    Op: Operation,
{
    /// Register an effect request for later continuing with [`Self::resolve`]. Stores
    /// the effect's handle under an ID, and returns the ID and operation the request
    /// was carrying.
    ///
    /// # Panics
    ///
    /// Panics if the lock around the underlying storage was poisoned, or if the
    /// storage index exceeds the available ID space.
    pub fn register(&self, request: Request<Op>) -> (EffectId<Op::Output>, Op) {
        let (operation, handle) = request.split();
        let id = self
            .requests
            .lock()
            .expect("registry lock poisoned")
            .insert(handle);

        (id, operation)
    }

    /// Resolve an effect stored under `id` with an `output`.
    /// If the request is expected to resolve multiple times it will be reinserted.
    ///
    /// # Errors
    ///
    /// Returns an Err(ResolveError) if the request cannot be resolved.
    ///
    /// # Panics
    ///
    /// Panics if the lock around the underlying storage was poisoned.
    pub fn resolve(
        &self,
        id: EffectId<Op::Output>,
        output: Op::Output,
    ) -> Result<(), ResolveError> {
        let mut handle = self
            .requests
            .lock()
            .expect("registry lock poisoned")
            .take(id)
            .ok_or_else(|| ResolveError::NotFound(id.into_raw()))?;

        let result = handle.resolve(output);
        let should_reinsert = result.is_ok() && matches!(handle, RequestHandle::Many(_));

        let mut requests = self.requests.lock().expect("registry lock poisoned");
        if should_reinsert {
            requests.reinsert(id, handle);
        } else {
            requests.remove(id);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct TestOp;

    impl Operation for TestOp {
        type Output = char;
    }

    #[test]
    fn register_does_not_require_clone() {
        let registry = Registry::<TestOp>::default();
        let (id, operation) = registry.register(Request::resolves_once(TestOp, |_| {}));

        assert_eq!(operation, TestOp);
        registry.resolve(id, 'a').unwrap();
    }

    #[test]
    fn stale_id_does_not_resolve_reused_slot() {
        let registry = Registry::<TestOp>::default();

        let (stale_id, _) = registry.register(Request::resolves_once(TestOp, |_| {}));
        registry.resolve(stale_id, 'a').unwrap();

        let resolved = Arc::new(AtomicUsize::new(0));
        let resolved_clone = resolved.clone();
        let (next_id, _) = registry.register(Request::resolves_once(TestOp, move |_| {
            resolved_clone.fetch_add(1, Ordering::Relaxed);
        }));

        // slot is reused
        assert_eq!(stale_id.index(), next_id.index());

        // generation is different
        assert_ne!(stale_id.generation(), next_id.generation());
        assert_ne!(stale_id.into_raw(), next_id.into_raw());

        // resolving with the stale_id results in a not found
        assert!(matches!(
            registry.resolve(stale_id, 'a'),
            Err(ResolveError::NotFound(_))
        ));
        assert_eq!(resolved.load(Ordering::Relaxed), 0);

        registry.resolve(next_id, 'a').unwrap();
        assert_eq!(resolved.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn many_handles_are_reinserted_after_successful_resolution() {
        let registry = Registry::<TestOp>::default();
        let resolved = Arc::new(Mutex::new(Vec::new()));
        let resolved_clone = resolved.clone();

        let (id, _) = registry.register(Request::resolves_many_times(TestOp, move |output| {
            resolved_clone.lock().unwrap().push(output);
            Ok(())
        }));

        registry.resolve(id, 'a').unwrap();
        registry.resolve(id, 'b').unwrap();

        assert_eq!(*resolved.lock().unwrap(), vec!['a', 'b']);
    }

    #[test]
    fn finished_many_handle_is_removed() {
        let registry = Registry::<TestOp>::default();
        let (id, _) = registry.register(Request::resolves_many_times(TestOp, |_| Err(())));

        assert!(matches!(
            registry.resolve(id, 'a'),
            Err(ResolveError::FinishedMany)
        ));
        assert!(matches!(
            registry.resolve(id, 'a'),
            Err(ResolveError::NotFound(_))
        ));
    }
}
