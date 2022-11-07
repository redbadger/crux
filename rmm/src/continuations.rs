use std::{collections::HashMap, sync::RwLock};
use uuid::Uuid;

use crate::{Request, RequestBody};

type Store<ResponseData, Message> =
    HashMap<[u8; 16], Box<dyn FnOnce(ResponseData) -> Message + Sync + Send>>;
pub struct ContinuationStore<ResponseData, Message>(RwLock<Store<ResponseData, Message>>);

impl<ResponseData, Message> Default for ContinuationStore<ResponseData, Message> {
    fn default() -> Self {
        Self(RwLock::new(HashMap::new()))
    }
}

impl<ResponseData, Message> ContinuationStore<ResponseData, Message> {
    pub fn pause<F>(&self, body: RequestBody, msg: F) -> Request
    where
        F: FnOnce(ResponseData) -> Message + Sync + Send + 'static,
    {
        let uuid = *Uuid::new_v4().as_bytes();
        println!("pause uuid {:x?}, length {}", uuid, uuid.len());

        self.0.write().unwrap().insert(uuid, Box::new(msg));

        Request {
            uuid: uuid.to_vec(),
            body,
        }
    }

    pub fn resume(&self, uuid: Vec<u8>, data: ResponseData) -> Message {
        println!("resume uuid {:x?}, length {}", uuid, uuid.len());
        let cont = self.0.write().unwrap().remove(&uuid[..]).unwrap();

        cont(data)
    }
}
