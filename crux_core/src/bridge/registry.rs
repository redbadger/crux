use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Mutex,
};

use uuid::Uuid;

use super::Request;
use crate::bridge::request_serde::ResolveBytes;
use crate::core::ResolveError;
use crate::Effect;

type Store<T> = HashMap<[u8; 16], T>;

pub(crate) struct ResolveRegistry(Mutex<Store<ResolveBytes>>);

impl Default for ResolveRegistry {
    fn default() -> Self {
        Self(Mutex::new(Store::new()))
    }
}

impl ResolveRegistry {
    pub(crate) fn register<Eff>(&self, effect: Eff) -> Request<Eff::Ffi>
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

    pub(crate) fn resume(&self, uuid: &[u8], body: &[u8]) -> Result<(), ResolveError> {
        let mut registry_lock = self.0.lock().expect("Registry Mutex poisoned");

        let entry = {
            let mut uuid_buf = [0; 16];
            uuid_buf.copy_from_slice(uuid);

            registry_lock.entry(uuid_buf)
        };

        let Entry::Occupied(mut entry) = entry else {
            panic!("Request with UUID {uuid:?} not found.");
        };

        let resolve = entry.get_mut();

        // FIXME bubble up the error
        resolve.resolve(body)
    }
}
