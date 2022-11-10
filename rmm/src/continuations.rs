use std::{collections::HashMap, sync::RwLock};
use uuid::Uuid;

use crate::{Command, Request, RequestBody, Response, ResponseBody};

type Store<Message> = HashMap<[u8; 16], Box<dyn FnOnce(ResponseBody) -> Message + Sync + Send>>;
pub struct ContinuationStore<Message>(RwLock<Store<Message>>);

impl<Message> Default for ContinuationStore<Message> {
    fn default() -> Self {
        Self(RwLock::new(HashMap::new()))
    }
}

impl<Message> ContinuationStore<Message> {
    pub fn pause<F>(&self, cmd: Command<F, Message>) -> Request
    where
        F: FnOnce(ResponseBody) -> Message + Sync + Send + 'static,
    {
        let Command {
            body,
            msg_constructor,
        } = cmd;
        let uuid = *Uuid::new_v4().as_bytes();

        self.0
            .write()
            .unwrap()
            .insert(uuid, Box::new(msg_constructor));

        Request {
            uuid: uuid.to_vec(),
            body,
        }
    }

    pub fn resume(&self, response: Response) -> Message {
        let Response { uuid, body } = response;
        let cont = self.0.write().unwrap().remove(&uuid[..]).unwrap();

        cont(body)
    }
}
