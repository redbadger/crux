use crate::Request;
use std::{collections::HashMap, sync::RwLock};
use uuid::Uuid;

type Store<T> = HashMap<[u8; 16], Box<dyn FnOnce(String) -> T + Sync + Send>>;

pub struct Time<Msg> {
    continuations: RwLock<Store<Msg>>,
}

impl<Msg> Default for Time<Msg> {
    fn default() -> Self {
        Self {
            continuations: RwLock::new(HashMap::new()),
        }
    }
}

impl<Msg> Time<Msg> {
    pub fn get<F>(&self, msg: F) -> Request
    where
        F: Sync + Send + FnOnce(String) -> Msg + 'static,
    {
        let uuid = *Uuid::new_v4().as_bytes();

        self.continuations
            .write()
            .unwrap()
            .insert(uuid, Box::new(msg));

        Request::Time {
            uuid: uuid.to_vec(),
        }
    }

    pub fn receive(&self, uuid: &[u8], data: String) -> Msg {
        let mut continuations = self.continuations.write().unwrap();
        let f = continuations.remove(uuid).unwrap();

        f(data)
    }
}
