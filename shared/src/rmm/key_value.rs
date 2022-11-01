use crate::Request;
use std::{collections::HashMap, sync::RwLock};
use uuid::Uuid;

type ReadStore<T> = HashMap<[u8; 16], Box<dyn FnOnce(Option<Vec<u8>>) -> T + Sync + Send>>;
type WriteStore<T> = HashMap<[u8; 16], Box<dyn FnOnce(bool) -> T + Sync + Send>>;

pub struct KeyValue<Msg> {
    reads: RwLock<ReadStore<Msg>>,
    writes: RwLock<WriteStore<Msg>>,
}

impl<Msg> Default for KeyValue<Msg> {
    fn default() -> Self {
        Self {
            reads: RwLock::new(HashMap::new()),
            writes: RwLock::new(HashMap::new()),
        }
    }
}

impl<Msg> KeyValue<Msg> {
    pub fn write<F>(&self, key: String, bytes: Vec<u8>, msg: F) -> Request
    where
        F: Sync + Send + FnOnce(bool) -> Msg + 'static,
    {
        let uuid = *Uuid::new_v4().as_bytes();

        self.writes.write().unwrap().insert(uuid, Box::new(msg));

        Request::KVWrite {
            uuid: uuid.to_vec(),
            key,
            bytes,
        }
    }

    pub fn written(&self, uuid: &[u8], result: bool) -> Msg {
        let mut writes = self.writes.write().unwrap();
        let f = writes.remove(uuid).unwrap();

        f(result)
    }

    pub fn read<F>(&self, key: String, msg: F) -> Request
    where
        F: Sync + Send + FnOnce(Option<Vec<u8>>) -> Msg + 'static,
    {
        let uuid = *Uuid::new_v4().as_bytes();

        self.reads.write().unwrap().insert(uuid, Box::new(msg));

        Request::KVRead {
            uuid: uuid.to_vec(),
            key,
        }
    }

    pub fn receive_read(&self, uuid: &[u8], bytes: Option<Vec<u8>>) -> Msg {
        let mut reads = self.reads.write().unwrap();
        let f = reads.remove(uuid).unwrap();

        f(bytes)
    }
}
