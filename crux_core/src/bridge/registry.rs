use std::sync::Mutex;

use facet::Facet;
use serde::{Deserialize, Serialize};
use slab::Slab;

use super::{BridgeError, FfiFormat, Request};
use crate::bridge::request_serde::ResolveSerialized;
use crate::{EffectFFI, ResolveError};

#[derive(Facet, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[facet(transparent)]
pub struct EffectId(pub u32);

pub struct ResolveRegistry<T: FfiFormat>(Mutex<Slab<ResolveSerialized<T>>>);

impl<T: FfiFormat> Default for ResolveRegistry<T> {
    fn default() -> Self {
        Self(Mutex::new(Slab::with_capacity(1024)))
    }
}

impl<T: FfiFormat> ResolveRegistry<T> {
    /// Register an effect for future continuation, when it has been processed
    /// and output given back to the core.
    ///
    /// The `effect` will be serialized into its FFI counterpart before being stored
    /// and wrapped in a [`Request`].
    // used in docs/internals/bridge.md
    // ANCHOR: register
    pub fn register<Eff>(&self, effect: Eff) -> Request<Eff::Ffi>
    where
        Eff: EffectFFI,
    {
        let (effect, resolve) = effect.serialize();

        let id = self
            .0
            .lock()
            .expect("Registry Mutex poisoned.")
            .insert(resolve);

        Request {
            id: EffectId(id.try_into().expect("EffectId overflow")),
            effect,
        }
    }
    // ANCHOR_END: register

    /// Resume a previously registered effect. This may fail, either because `EffectId` wasn't
    /// found or because this effect was not expected to be resumed again.
    pub fn resume(&self, id: EffectId, response: &[u8]) -> Result<(), BridgeError<T>> {
        let mut registry_lock = self.0.lock().expect("Registry Mutex poisoned");

        let entry = registry_lock.get_mut(id.0 as usize);

        let Some(entry) = entry else {
            return Err(BridgeError::ProcessResponse(ResolveError::NotFound(id)));
        };

        let resolved = entry.resolve(response);

        if let ResolveSerialized::Never = entry {
            registry_lock.remove(id.0 as usize);
        }

        resolved
    }
}
