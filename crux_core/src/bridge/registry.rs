use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Mutex,
};

use uuid::Uuid;

use super::Request;
use crate::{
    steps::{Resolve, ResolveError},
    Effect,
};

type Store<T> = HashMap<[u8; 16], T>;

pub(crate) struct ResolveRegistry<'de>(Mutex<Store<Resolve<&'de [u8]>>>);

impl<'de> Default for ResolveRegistry<'de> {
    fn default() -> Self {
        Self(Mutex::new(Store::new()))
    }
}

impl<'de> ResolveRegistry<'de> {
    pub(crate) fn register<Eff>(&self, effect: Eff) -> Request<Eff>
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

    pub(crate) fn resume(&self, uuid: &[u8], body: &'de [u8]) -> Result<(), ResolveError> {
        let mut registry_lock = self.0.lock().expect("Registry Mutex poisoned");

        let entry = {
            let mut uuid_buf = [0; 16];
            uuid_buf.copy_from_slice(uuid);

            registry_lock.entry(uuid_buf)
        };

        let Entry::Occupied(mut entry) = entry else {
            panic!("Step with UUID {uuid:?} not found.");
        };

        let resolve = entry.get_mut();

        // FIXME bubble up the error
        resolve.resolve(body)
    }
}
