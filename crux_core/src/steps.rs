use crate::Request;
use std::{collections::HashMap, sync::Mutex};
use uuid::Uuid;

/// TODO: docs
pub(crate) struct Step<Ef> {
    pub(crate) effect: Ef,
    pub(crate) resolve: Option<Resolve>,
}

pub(crate) type Resolve = Box<dyn Fn(&[u8]) + Send>;

impl<Ef> Step<Ef> {
    pub fn new<F>(effect: Ef, resume: F) -> Self
    where
        F: Fn(&[u8]) + Send + 'static,
    {
        Self {
            effect,
            resolve: Some(Box::new(resume)),
        }
    }

    pub fn once(effect: Ef) -> Self {
        Self {
            effect,
            resolve: None,
        }
    }

    pub fn map_effect<NewEffect, F>(self, f: F) -> Step<NewEffect>
    where
        F: Fn(Ef) -> NewEffect + Sync + Send + Copy + 'static,
        Ef: 'static,
        NewEffect: 'static,
    {
        Step {
            effect: f(self.effect),
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
        let Step { effect, resolve } = step;

        let uuid = *Uuid::new_v4().as_bytes();

        if let Some(resolve) = resolve {
            self.0
                .lock()
                .expect("Continuation Mutex poisoned.")
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
            .expect("Continuation Mutex poisoned.")
            .0
            .remove(uuid)
            .unwrap_or_else(|| panic!("Continuation with UUID {:?} not found.", uuid));

        (*resolve)(body);
    }
}
