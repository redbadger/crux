use crate::command::{Command, Resolve};
use crate::Request;
use std::{collections::HashMap, sync::Mutex};
use uuid::Uuid;

struct Store(HashMap<[u8; 16], Resolve>);

pub(crate) struct ContinuationStore(Mutex<Store>);

impl Default for ContinuationStore {
    fn default() -> Self {
        Self(Mutex::new(Store(HashMap::new())))
    }
}

impl ContinuationStore {
    pub(crate) fn pause<Ef>(&self, cmd: Command<Ef>) -> Request<Ef> {
        let Command { effect, resolve } = cmd;

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

    pub(crate) fn resume(&self, uuid: &[u8], body: Vec<u8>) {
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
