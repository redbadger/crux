use serde::{Deserialize, Serialize};
use std::{collections::HashMap, marker::PhantomData, sync::RwLock};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Envelope<T> {
    pub uuid: Vec<u8>,
    pub body: T,
}

pub(crate) type ContinuationStore<Response, Message> =
    HashMap<[u8; 16], Box<dyn FnOnce(Response) -> Message + Sync + Send>>;

pub struct Capability<Msg, Req, Res> {
    req: PhantomData<Req>,
    continuations: RwLock<ContinuationStore<Res, Msg>>,
}

impl<Msg, Req, Res> Default for Capability<Msg, Req, Res> {
    fn default() -> Self {
        Self {
            continuations: RwLock::new(HashMap::new()),
            req: PhantomData,
        }
    }
}

impl<Msg, Req, Res> Capability<Msg, Req, Res> {
    pub fn request<F>(&self, body: Req, msg: F) -> Envelope<Req>
    where
        F: Sync + Send + FnOnce(Res) -> Msg + 'static,
    {
        let uuid = *Uuid::new_v4().as_bytes();

        self.continuations
            .write()
            .unwrap()
            .insert(uuid, Box::new(msg));

        Envelope {
            uuid: uuid.to_vec(),
            body,
        }
    }

    pub fn response(&self, res: Envelope<Res>) -> Msg {
        let mut continuations = self.continuations.write().unwrap();
        let f = continuations.remove(&res.uuid[..]).unwrap();

        f(res.body)
    }
}
