use super::EventName;

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
    /// Starts before `App::update`. The span covers that call and its returned
    /// command. `caused_by` names the command which emitted this event, or ROOT
    /// for a shell event. This is a causal link, not an owning parent: the
    /// originating command may already have ended.
    fn command_started(
        &self,
        _context_id: RuntimeContextId,
        _caused_by: RuntimeContextId,
        _name: &'static str,
    ) {
    }

    /// The returned command's owned work has been cleaned up and its queued
    /// outputs forwarded (or discarded during cancellation/teardown).
    /// Commands returned by later updates have independent lifetimes.
    /// Completion and cancellation are not distinguished. Neither implies
    /// that external shell work has stopped.
    fn command_ended(&self, _context_id: RuntimeContextId) {}

    /// Core is dispatching a request or stream to the shell. The span measures the
    /// command's wait, not the lifetime of external shell work. Waits abandoned
    /// before dispatch are discarded without starting a span.
    fn effect_started(
        &self,
        _context_id: RuntimeContextId,
        _parent_context_id: RuntimeContextId,
        _name: &'static str,
        _kind: super::OperationKind,
    ) {
    }

    /// The waiting request completed, or the stream terminated, or its waiting
    /// future/stream was dropped. Reported exactly once per observed effect.
    fn effect_ended(&self, _context_id: RuntimeContextId) {}

    /// The command consumed one stream response. Buffered but unconsumed
    /// responses have no marker, nor do items consumed before dispatch.
    /// Payload values are not recorded.
    fn stream_item(&self, _context_id: RuntimeContextId) {}

    /// Core dispatches a notification. Its originating command may already
    /// have finished; notifications do not extend command lifetimes.
    fn notification(&self, _parent_context_id: RuntimeContextId, _name: &'static str) {}
}

/// Core's optional observation configuration. Capturing the naming function
/// here keeps the `EventName` bound out of unobserved apps and command composition.
pub struct RuntimeObservation<Event> {
    pub(super) observer: Arc<dyn CoreRuntimeObserver + Send + Sync>,
    next_id: AtomicU64,
    event_name: fn(&Event) -> &'static str,
}

impl<Event> RuntimeObservation<Event> {
    pub(crate) fn new(observer: impl CoreRuntimeObserver + Send + Sync + 'static) -> Self
    where
        Event: EventName,
    {
        Self {
            observer: Arc::new(observer),
            next_id: AtomicU64::new(1),
            event_name: Event::event_name,
        }
    }

    pub(crate) fn start_command(
        &self,
        event: &Event,
        caused_by: Option<RuntimeContextId>,
    ) -> CommandObservation {
        let id = self.next_context_id();
        self.observer.command_started(
            id,
            caused_by.unwrap_or(RuntimeContextId::ROOT),
            (self.event_name)(event),
        );
        CommandObservation {
            id,
            observer: self.observer.clone(),
        }
    }

    pub(super) fn next_context_id(&self) -> RuntimeContextId {
        RuntimeContextId(self.next_id.fetch_add(1, Ordering::Relaxed))
    }
}

/// Owned by the update's command host. Dropping this guard closes its span.
pub struct CommandObservation {
    pub(crate) id: RuntimeContextId,
    observer: Arc<dyn CoreRuntimeObserver + Send + Sync>,
}

impl Drop for CommandObservation {
    fn drop(&mut self) {
        self.observer.command_ended(self.id);
    }
}
