use std::{
    collections::{hash_map::Entry, HashMap},
    fmt,
    sync::Mutex,
};

use uuid::Uuid;

use crate::Request;

pub(crate) struct Step<T> {
    pub(crate) payload: T, // T is an Operation first, then Effect once mapped
    pub(crate) resolve: Option<Resolve>,
}

type OnceFn = dyn FnOnce(&[u8]) + Send;
type ManyFn = dyn Fn(&[u8]) -> Result<(), ()> + Send;

pub(crate) enum Resolve {
    Once(Box<OnceFn>),
    Many(Box<ManyFn>),
}

impl<T> Step<T> {
    pub fn resolves_never(payload: T) -> Self {
        Self {
            payload,
            resolve: None,
        }
    }

    pub fn resolves_once<F>(payload: T, resume: F) -> Self
    where
        F: FnOnce(&[u8]) + Send + 'static,
    {
        Self {
            payload,
            resolve: Some(Resolve::Once(Box::new(resume))),
        }
    }

    pub fn resolves_many_times<F>(payload: T, resume: F) -> Self
    where
        F: Fn(&[u8]) -> Result<(), ()> + Send + 'static,
    {
        Self {
            payload,
            resolve: Some(Resolve::Many(Box::new(resume))),
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

impl<T> fmt::Debug for Step<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Step")
            .field("payload", &self.payload)
            .finish_non_exhaustive()
    }
}

type Store = HashMap<[u8; 16], Resolve>;

pub(crate) struct StepRegistry(Mutex<Store>);

impl Default for StepRegistry {
    fn default() -> Self {
        Self(Mutex::new(Store::new()))
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
                .insert(uuid, resolve);
        }

        Request {
            uuid: uuid.to_vec(),
            effect,
        }
    }

    pub(crate) fn resume(&self, uuid: &[u8], body: &[u8]) {
        let mut registry_lock = self.0.lock().expect("Step Mutex poisoned");

        let entry = {
            let mut uuid_buf = [0; 16];
            uuid_buf.copy_from_slice(uuid);

            registry_lock.entry(uuid_buf)
        };

        let Entry::Occupied(entry) = entry else {
            panic!("Step with UUID {uuid:?} not found.");
        };

        match entry.get() {
            Resolve::Once(_) => {
                let Resolve::Once(resolve) = entry.remove() else {
                    unreachable!()
                };
                drop(registry_lock);
                resolve(body)
            }
            Resolve::Many(resolve) => {
                match resolve(body) {
                    Ok(_) => {}
                    Err(_) => {
                        // The associated task has ended so clean up our state.
                        // We _probably_ want a way to let the shell know about this...
                        // But I'm going to postpone figuring out how to do that.
                        entry.remove();
                    }
                }
            }
        }
    }
}
