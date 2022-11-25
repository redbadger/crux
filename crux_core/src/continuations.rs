use crate::command::{Callback, Command};
use crate::{Request, Response};

use std::{collections::HashMap, sync::RwLock};
use uuid::Uuid;

struct Store<Event>(HashMap<[u8; 16], Box<dyn Callback<Event> + Send + Sync>>);

pub(crate) struct ContinuationStore<Event>(RwLock<Store<Event>>);

impl<Event> Default for ContinuationStore<Event> {
    fn default() -> Self {
        Self(RwLock::new(Store(HashMap::new())))
    }
}

impl<Event> ContinuationStore<Event> {
    pub(crate) fn pause<Effect>(&self, cmd: Command<Effect, Event>) -> Request<Effect> {
        let Command { effect, resolve } = cmd;

        let uuid = *Uuid::new_v4().as_bytes();
        if let Some(resolve) = resolve {
            self.0
                .write()
                .expect("Continuation RwLock poisoned.")
                .0
                .insert(uuid, resolve);
        }

        Request {
            uuid: uuid.to_vec(),
            effect,
        }
    }

    pub(crate) fn resume(&self, response: Response) -> Event {
        let Response { uuid, body } = response;

        let resolve = self
            .0
            .write()
            .expect("Continuation RwLock poisoned.")
            .0
            .remove(&uuid[..])
            .unwrap_or_else(|| panic!("Continuation with UUID {:?} not found.", uuid));

        resolve.call(body)
    }
}
