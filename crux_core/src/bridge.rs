use std::{collections::HashMap, sync::Mutex};

use crate::{steps::Step, Request};

type Store = HashMap<[u8; 16], bool>;

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

struct Bridge;

impl Bridge {
    /// Receive an event from the shell.
    ///
    /// The `event` is serialized and will be deserialized by the core before it's passed
    /// to your app.
    pub fn process_event<'de>(&self, event: &'de [u8]) -> Vec<u8>
    where
        <A as App>::Event: Deserialize<'de>,
    {
        self.process(None, event)
    }

    /// Receive a response to a capability request from the shell.
    ///
    /// The `output` is serialized capability output. It will be deserialized by the core.
    /// The `uuid` MUST match the `uuid` of the effect that triggered it, else the core will panic.
    pub fn handle_response<'de>(&self, uuid: &[u8], output: &'de [u8]) -> Vec<u8>
    where
        <A as App>::Event: Deserialize<'de>,
    {
        self.process(Some(uuid), output)
    }

    fn process<'de>(&self, uuid: Option<&[u8]>, data: &'de [u8]) -> Vec<u8>
    where
        <A as App>::Event: Deserialize<'de>,
    {
        match uuid {
            None => {
                let shell_event = bcs::from_bytes(data).expect("Message deserialization failed.");
                let mut model = self.model.write().expect("Model RwLock was poisoned.");
                self.app.update(shell_event, &mut model, &self.capabilities);
            }
            Some(uuid) => {
                self.step_registry.resume(uuid, data);
            }
        }

        self.executor.run_all();

        while let Some(capability_event) = self.capability_events.receive() {
            let mut model = self.model.write().expect("Model RwLock was poisoned.");
            self.app
                .update(capability_event, &mut model, &self.capabilities);
            drop(model);
            self.executor.run_all();
        }

        let requests = self
            .steps
            .drain()
            .map(|c| self.step_registry.register(c))
            .collect::<Vec<_>>();

        bcs::to_bytes(&requests).expect("Request serialization failed.")
    }
}
