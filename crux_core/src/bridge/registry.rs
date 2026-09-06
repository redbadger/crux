use std::collections::HashMap;
use std::sync::Mutex;

use facet::Facet;
use serde::{Deserialize, Serialize};

use super::{BridgeError, FfiFormat, Request};
use crate::bridge::request_serde::ResolveSerialized;
use crate::{EffectFFI, RequestKind, ResolveError};

/// Identifies one request across the FFI boundary, for as long as anything
/// could still refer to it.
///
/// Ids are issued in ascending order and are not reused when a request
/// completes, so an id that has been resolved stays unusable rather than being
/// handed to some unrelated later request.
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Facet, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[facet(transparent)]
pub struct EffectId(pub u32);

pub struct ResolveRegistry<T: FfiFormat>(Mutex<Outstanding<T>>);

/// The requests the shell could still resolve, keyed by the id it was given.
struct Outstanding<T: FfiFormat> {
    entries: HashMap<u32, ResolveSerialized<T>>,
    next_id: u32,
}

impl<T: FfiFormat> Outstanding<T> {
    /// Issue the next id.
    ///
    /// Ids ascend rather than filling gaps, so resolving a completed request is
    /// a lookup miss instead of a hit on whichever request happened to inherit
    /// its storage. The counter wraps after `u32::MAX` requests; ids still
    /// outstanding are stepped over, so a live request can never be displaced
    /// even then.
    fn issue_id(&mut self) -> u32 {
        loop {
            let id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1);

            if !self.entries.contains_key(&id) {
                return id;
            }
        }
    }
}

impl<T: FfiFormat> Default for ResolveRegistry<T> {
    fn default() -> Self {
        Self(Mutex::new(Outstanding {
            entries: HashMap::new(),
            next_id: 0,
        }))
    }
}

impl<T: FfiFormat> ResolveRegistry<T> {
    /// Register an effect for future continuation, when it has been processed
    /// and output given back to the core.
    ///
    /// The `effect` will be serialized into its FFI counterpart before being stored
    /// and wrapped in a [`Request`].
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex has been poisoned
    // ANCHOR: register
    pub fn register<Eff>(&self, effect: Eff) -> Request<Eff::Ffi>
    where
        Eff: EffectFFI,
    {
        let (effect, resolve) = effect.serialize();
        let kind = resolve.kind();

        let id = {
            let mut outstanding = self.0.lock().expect("Registry Mutex poisoned.");
            let id = outstanding.issue_id();

            // A request that cannot be resolved has nothing worth keeping: storing
            // one would add an entry per fire-and-forget effect — every render, for
            // the life of the process — that nothing would ever remove.
            if kind != RequestKind::Notify {
                outstanding.entries.insert(id, resolve);
            }

            id
        };

        Request {
            id: EffectId(id),
            effect,
        }
    }
    // ANCHOR_END: register

    /// Resume a previously registered effect.
    ///
    /// Fails with [`ResolveError::NotFound`] if `id` is not outstanding —
    /// because it was never issued, because it has already been resolved, or
    /// because it belongs to a request that never expected a response.
    ///
    /// # Errors
    ///
    /// Returns `BridgeError` if the stored request could not be resolved.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex has been poisoned
    pub fn resume(&self, id: EffectId, response: &[u8]) -> Result<(), BridgeError<T>> {
        let mut outstanding = self.0.lock().expect("Registry Mutex poisoned");

        let Some(entry) = outstanding.entries.get_mut(&id.0) else {
            return Err(BridgeError::ProcessResponse(ResolveError::NotFound(
                id.0.into(),
            )));
        };

        let resolved = entry.resolve(response);

        // A `Once` turns itself into a `Never` as it resolves: the request is
        // finished, and its id will not be issued again, so drop it.
        if matches!(entry, ResolveSerialized::Never) {
            outstanding.entries.remove(&id.0);
        }

        resolved
    }
}

#[cfg(test)]
mod tests {
    use super::{Outstanding, ResolveSerialized};
    use crate::bridge::JsonFfiFormat;
    use std::collections::HashMap;

    fn outstanding(next_id: u32) -> Outstanding<JsonFfiFormat> {
        Outstanding {
            entries: HashMap::new(),
            next_id,
        }
    }

    #[test]
    fn ids_ascend_and_are_never_reused() {
        let mut outstanding = outstanding(0);

        let ids: Vec<_> = (0..4).map(|_| outstanding.issue_id()).collect();
        assert_eq!(ids, vec![0, 1, 2, 3]);

        // Finishing a request frees its entry, but not its id.
        outstanding.entries.remove(&1);

        assert_eq!(outstanding.issue_id(), 4);
    }

    #[test]
    fn wrapping_steps_over_outstanding_ids() {
        let mut outstanding = outstanding(u32::MAX);

        // Still awaiting a response on 0 and 1 when the counter comes round.
        outstanding.entries.insert(0, ResolveSerialized::Never);
        outstanding.entries.insert(1, ResolveSerialized::Never);

        assert_eq!(outstanding.issue_id(), u32::MAX);
        assert_eq!(
            outstanding.issue_id(),
            2,
            "wrapping displaced a request that was still outstanding"
        );
    }
}
