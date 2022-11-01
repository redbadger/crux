use super::cmd::Store;
use crate::Request;
use std::{collections::HashMap, sync::RwLock};
use uuid::Uuid;

pub struct Platform<Msg> {
    continuations: RwLock<Store<String, Msg>>,
}

impl<Msg> Default for Platform<Msg> {
    fn default() -> Self {
        Self {
            continuations: RwLock::new(HashMap::new()),
        }
    }
}

impl<Msg> Platform<Msg> {
    pub fn get<F>(&self, msg: F) -> Request
    where
        F: Sync + Send + FnOnce(String) -> Msg + 'static,
    {
        let uuid = *Uuid::new_v4().as_bytes();

        self.continuations
            .write()
            .unwrap()
            .insert(uuid, Box::new(msg));

        Request::Platform {
            uuid: uuid.to_vec(),
        }
    }

    pub fn receive(&self, uuid: &[u8], data: String) -> Msg {
        let mut continuations = self.continuations.write().unwrap();
        let f = continuations.remove(uuid).unwrap();

        f(data)
    }
}
