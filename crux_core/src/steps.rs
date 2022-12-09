use crate::Request;
use std::{collections::HashMap, sync::Mutex};
use uuid::Uuid;

/// TODO: docs
pub(crate) struct Step<T> {
    pub(crate) payload: T, // T is an Operation first, then Effect once mapped
    pub(crate) resolve: Option<Resolve>,
}

pub(crate) type Resolve = Box<dyn Fn(&[u8]) + Send>;

impl<T> Step<T> {
    pub fn new<F>(payload: T, resume: F) -> Self
    where
        F: Fn(&[u8]) + Send + 'static,
    {
        Self {
            payload,
            resolve: Some(Box::new(resume)),
        }
    }

    pub fn once(payload: T) -> Self {
        Self {
            payload,
            resolve: None,
        }
    }

    pub fn map_effect<Ef, F>(self, f: F) -> Step<Ef>
    where
        F: Fn(T) -> Ef + Sync + Send + Copy + 'static,
        T: 'static,
        Ef: 'static,
    {
        Step {
            payload: f(self.payload),
            resolve: self.resolve,
        }
    }
}

struct Store(HashMap<[u8; 16], Resolve>);

pub(crate) struct StepRegistry(Mutex<Store>);

impl Default for StepRegistry {
    fn default() -> Self {
        Self(Mutex::new(Store(HashMap::new())))
    }
}

impl StepRegistry {
    pub(crate) fn register<Ef>(&self, step: Step<Ef>) -> Request<Ef> {
        let Step {
            payload: effect,
            resolve,
        } = step;

        let uuid = *Uuid::new_v4().as_bytes();

        if let Some(resolve) = resolve {
            self.0
                .lock()
                .expect("Step Mutex poisoned.")
                .0
                .insert(uuid, resolve);
        }

        Request {
            uuid: uuid.to_vec(),
            effect,
        }
    }

    pub(crate) fn resume(&self, uuid: &[u8], body: &[u8]) {
        let resolve = self
            .0
            .lock()
            .expect("Step Mutex poisoned.")
            .0
            .remove(uuid)
            .unwrap_or_else(|| panic!("Step with UUID {uuid:?} not found."));

        (*resolve)(body);
    }
}
