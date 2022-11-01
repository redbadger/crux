use crate::Request;
use std::{collections::HashMap, sync::RwLock};
use uuid::Uuid;

type Store<T> = HashMap<[u8; 16], Box<dyn FnOnce(Vec<u8>) -> T + Sync + Send>>;

pub struct Http<Msg> {
    continuations: RwLock<Store<Msg>>,
}

impl<Msg> Default for Http<Msg> {
    fn default() -> Self {
        Self {
            continuations: RwLock::new(HashMap::new()),
        }
    }
}

impl<Msg> Http<Msg> {
    pub fn get<F>(&self, url: String, msg: F) -> Request
    where
        F: Sync + Send + FnOnce(Vec<u8>) -> Msg + 'static,
    {
        let uuid = *Uuid::new_v4().as_bytes();

        self.continuations
            .write()
            .unwrap()
            .insert(uuid, Box::new(msg));

        Request::Http {
            uuid: uuid.to_vec(),
            url,
        }
    }

    pub fn receive(&self, uuid: &[u8], data: Vec<u8>) -> Msg {
        let mut continuations = self.continuations.write().unwrap();
        let f = continuations.remove(uuid).unwrap();

        f(data)
    }
}
