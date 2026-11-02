use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

/// Identifies an execution context in the Crux runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RuntimeContextId(pub u64);

impl RuntimeContextId {
    pub const ROOT: Self = Self(0);
}

/// Synchronous runtime observations; implementations supply timestamps.
///
/// Callbacks may run while Core holds runtime locks, including during teardown.
/// Implementations should not re-enter Core or panic.
pub trait CoreRuntimeObserver {
    fn event_started(
        &self,
        _context_id: RuntimeContextId,
        _parent_context_id: RuntimeContextId,
        _name: &'static str,
    ) {
    }

    /// The event's owned command subtree has been cleaned up. Normal completion
    /// waits for `Command::is_done()`; cancellation reports after subtree teardown.
    /// This does not distinguish successful completion from cancellation, or
    /// imply that work held by a shell has stopped.
    fn event_ended(&self, _context_id: RuntimeContextId) {}

    /// A request or stream crossed its originating event host. Effect-end
    /// tracking is not yet implemented.
    fn effect_started(
        &self,
        _context_id: RuntimeContextId,
        _parent_context_id: RuntimeContextId,
        _name: &'static str,
    ) {
    }

    fn notification(&self, _parent_context_id: RuntimeContextId, _name: &'static str) {}
}

/// Observation state shared by Core and its observable command hosts.
/// Created only when an observer is installed; clones share the observer and
/// the same ID sequence.
#[derive(Clone)]
pub struct RuntimeObservationContext {
    pub(crate) observer: Arc<dyn CoreRuntimeObserver + Send + Sync>,
    pub(crate) shared_context_id_generator: ContextIdGenerator,
}

impl RuntimeObservationContext {
    pub(crate) fn new(observer: impl CoreRuntimeObserver + Send + Sync + 'static) -> Self {
        Self {
            observer: Arc::new(observer),
            shared_context_id_generator: ContextIdGenerator::default(),
        }
    }
}

#[derive(Clone)]
pub struct ContextIdGenerator {
    next_id: Arc<AtomicU64>,
}

impl Default for ContextIdGenerator {
    fn default() -> Self {
        Self {
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }
}

impl ContextIdGenerator {
    pub(crate) fn next_context_id(&self) -> RuntimeContextId {
        RuntimeContextId(self.next_id.fetch_add(1, Ordering::Relaxed))
    }
}
