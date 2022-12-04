use crate::command::{Callback, Command, Resolve};
use crate::Request;
use std::{collections::HashMap, sync::Mutex};
use uuid::Uuid;

struct Store<Ev>(HashMap<[u8; 16], Resolve<Ev>>);

pub(crate) struct ContinuationStore<Ev>(Mutex<Store<Ev>>);

impl<Ev> Default for ContinuationStore<Ev> {
    fn default() -> Self {
        Self(Mutex::new(Store(HashMap::new())))
    }
}

impl<Ev> ContinuationStore<Ev> {
    pub(crate) fn pause<Ef>(&self, cmd: Command<Ef, Ev>) -> Request<Ef> {
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

    pub(crate) fn resume(&self, uuid: &[u8], body: Vec<u8>) -> Option<Ev> {
        let resolve = self
            .0
            .lock()
            .expect("Continuation Mutex poisoned.")
            .0
            .remove(uuid)
            .unwrap_or_else(|| panic!("Continuation with UUID {:?} not found.", uuid));

        match resolve {
            Resolve::Event(callback) => Some(callback.call(body)),
            Resolve::Continue(callback) => {
                callback.call(body);
                None
            }
        }
    }
}
