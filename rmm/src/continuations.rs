use crate::{Command, Request, Response, ResponseBody};
use std::{collections::HashMap, sync::RwLock};
use uuid::Uuid;

type Store<Message> = HashMap<[u8; 16], Box<dyn FnOnce(ResponseBody) -> Message + Sync + Send>>;
pub struct ContinuationStore<Message>(RwLock<Store<Message>>);

impl<Message> Default for ContinuationStore<Message> {
    fn default() -> Self {
        Self(RwLock::new(HashMap::new()))
    }
}

impl<Message> ContinuationStore<Message> {
    pub fn pause(&self, cmd: Command<Message>) -> Request {
        let Command {
            body,
            msg_constructor,
        } = cmd;
        let uuid = *Uuid::new_v4().as_bytes();
        if let Some(msg_constructor) = msg_constructor {
            self.0.write().unwrap().insert(uuid, msg_constructor);
        }
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
