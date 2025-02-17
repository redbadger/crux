use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use slab::Slab;

use super::{BridgeError, Request};
use crate::bridge::request_serde::ResolveSerialized;
use crate::Effect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EffectId(pub u32);

pub struct ResolveRegistry(Mutex<Slab<ResolveSerialized>>);

impl Default for ResolveRegistry {
    fn default() -> Self {
        Self(Mutex::new(Slab::with_capacity(1024)))
    }
}

impl ResolveRegistry {
    /// Register an effect for future continuation, when it has been processed
    /// and output given back to the core.
    ///
    /// The `effect` will be serialized into its FFI counterpart before being stored
    /// and wrapped in a [`Request`].
    // used in docs/internals/bridge.md
    // ANCHOR: register
    pub fn register<Eff>(&self, effect: Eff) -> Request<Eff::Ffi>
    where
        Eff: Effect,
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

    /// Resume a previously registered effect. This may fail, either because EffectId wasn't
    /// found or because this effect was not expected to be resumed again.
    pub fn resume(
        &self,
        id: EffectId,
        body: &mut dyn erased_serde::Deserializer,
    ) -> Result<(), BridgeError> {
        let mut registry_lock = self.0.lock().expect("Registry Mutex poisoned");

        let entry = registry_lock.get_mut(id.0 as usize);

        let Some(entry) = entry else {
            // FIXME return an Err instead of panicking here.
            panic!("Request with {id:?} not found.");
        };

        let resolved = entry.resolve(body);

        if let ResolveSerialized::Never = entry {
            registry_lock.remove(id.0 as usize);
        }

        resolved
    }
}
