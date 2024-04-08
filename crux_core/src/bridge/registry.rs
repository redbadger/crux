use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Mutex,
};

use uuid::Uuid;

use super::Request;
use crate::bridge::request_serde::ResolveSerialized;
use crate::core::ResolveError;
use crate::Effect;

type Store<T> = HashMap<[u8; 16], T>;

pub struct ResolveRegistry(Mutex<Store<ResolveSerialized>>);

impl Default for ResolveRegistry {
    fn default() -> Self {
        Self(Mutex::new(Store::new()))
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
        let uuid = *Uuid::new_v4().as_bytes();
        let (effect, resolve) = effect.serialize();

        self.0
            .lock()
            .expect("Registry Mutex poisoned.")
            .insert(uuid, resolve);

        Request {
            uuid: uuid.to_vec(),
            effect,
        }
    }
    // ANCHOR_END: register

    /// Resume a previously registered effect. This may fail, either because UUID wasn't
    /// found or because this effect was not expected to be resumed again.
    pub fn resume(
        &self,
        uuid: &[u8],
        body: &mut dyn erased_serde::Deserializer,
    ) -> Result<(), ResolveError> {
        let mut registry_lock = self.0.lock().expect("Registry Mutex poisoned");

        let entry = {
            let mut uuid_buf = [0; 16];
            uuid_buf.copy_from_slice(uuid);

            registry_lock.entry(uuid_buf)
        };

        let Entry::Occupied(mut entry) = entry else {
            // FIXME return an Err instead of panicking here.
            panic!("Request with UUID {uuid:?} not found.");
        };

        let resolve = entry.get_mut();

        resolve.resolve(body)
    }
}
