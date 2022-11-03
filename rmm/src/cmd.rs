use std::{collections::HashMap, sync::RwLock};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    http::Http, key_value::KeyValueRead, key_value::KeyValueWrite, platform::Platform, time::Time,
};

struct ContinuationStore<ResponseData, Message>(
    HashMap<[u8; 16], Box<dyn FnOnce(ResponseData) -> Message + Sync + Send>>,
);

impl<ResponseData, Message> Default for ContinuationStore<ResponseData, Message> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<ResponseData, Message> ContinuationStore<ResponseData, Message> {
    pub fn pause<F>(&mut self, body: RequestBody, msg: F) -> Request
    where
        F: FnOnce(ResponseData) -> Message + Sync + Send + 'static,
    {
        let uuid = *Uuid::new_v4().as_bytes();

        self.0.insert(uuid, Box::new(msg));

        Request {
            uuid: uuid.to_vec(),
            body,
        }
    }

    fn resume(&mut self, uuid: Vec<u8>, data: ResponseData) -> Message {
        let cont = self.0.remove(&uuid[..]).unwrap();

        cont(data)
    }
}

pub struct Continuations<Msg> {
    pub http: RwLock<ContinuationStore<Vec<u8>, Msg>>,
    pub time: RwLock<ContinuationStore<String, Msg>>,
    pub key_value_read: RwLock<ContinuationStore<Option<Vec<u8>>, Msg>>,
    pub key_value_write: RwLock<ContinuationStore<bool, Msg>>,
    pub platform: RwLock<ContinuationStore<String, Msg>>,
}

impl<Msg> Default for Continuations<Msg> {
    fn default() -> Self {
        Self {
            http: Default::default(),
            time: Default::default(),
            key_value_read: Default::default(),
            key_value_write: Default::default(),
            platform: Default::default(),
        }
    }
}

// TODO consider whether these fields should be public
pub struct Cmd<'c, Msg> {
    continuations: Continuations<Msg>,
    pub http: Http<'c, Msg>,
    pub time: Time<'c, Msg>,
    pub key_value_read: KeyValueRead<'c, Msg>,
    pub key_value_write: KeyValueWrite<'c, Msg>,
    pub platform: Platform<'c, Msg>,
}

impl<'c, Msg> Default for Cmd<'c, Msg> {
    fn default() -> Self {
        let continuations = Continuations::default();

        Self {
            continuations,
            http: Http::new(&continuations),
            time: Time::new(&continuations),
            key_value_read: KeyValueRead::new(&continuations),
            key_value_write: KeyValueWrite::new(&continuations),
            platform: Platform::new(&continuations),
        }
    }
}

impl<'s, Msg> Cmd<'_, Msg> {
    pub fn resume(&mut self, response: Response) -> Msg {
        let Response { uuid, body } = response;

        match body {
            ResponseBody::Http(data) => {
                let mut cont = self.continuations.http.write().unwrap();
                cont.resume(uuid, data)
            }
            ResponseBody::Time(data) => {
                let mut cont = self.continuations.time.write().unwrap();
                cont.resume(uuid, data)
            }
            ResponseBody::Platform(data) => {
                let mut cont = self.continuations.platform.write().unwrap();
                cont.resume(uuid, data)
            }
            ResponseBody::KVRead(data) => {
                let mut cont = self.continuations.key_value_read.write().unwrap();
                cont.resume(uuid, data)
            }
            ResponseBody::KVWrite(data) => {
                let mut cont = self.continuations.key_value_write.write().unwrap();
                cont.resume(uuid, data)
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Request {
    uuid: Vec<u8>,
    body: RequestBody,
}

#[derive(Serialize, Deserialize)]
pub enum RequestBody {
    Time, // FIXME should be Envelope<()>, but serde struggles
    Http(String),
    Platform, // FIXME should be Envelope<()>, but serde struggles
    KVRead(String),
    KVWrite(String, Vec<u8>),
    Render,
}

pub struct Response {
    uuid: Vec<u8>,
    body: ResponseBody,
}

#[derive(Serialize, Deserialize)]
pub enum ResponseBody {
    Http(Vec<u8>),
    Time(String),
    Platform(String),
    KVRead(Option<Vec<u8>>),
    KVWrite(bool),
}
