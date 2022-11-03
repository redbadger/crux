use serde::{Deserialize, Serialize};

use super::{
    http::Http, key_value::KeyValueRead, key_value::KeyValueWrite, platform::Platform, time::Time,
};

// TODO consider whether these fields should be public
pub struct Cmd<Msg> {
    pub http: Http<Msg>,
    pub time: Time<Msg>,
    pub key_value_read: KeyValueRead<Msg>,
    pub key_value_write: KeyValueWrite<Msg>,
    pub platform: Platform<Msg>,
}

impl<Msg> Default for Cmd<Msg> {
    fn default() -> Self {
        Self {
            http: Http::default(),
            time: Time::default(),
            key_value_read: KeyValueRead::default(),
            key_value_write: KeyValueWrite::default(),
            platform: Platform::default(),
        }
    }
}

impl<Msg> Cmd<Msg> {
    pub fn resume(&self, response: Response) -> Msg {
        let Response { uuid, body } = response;

        match body {
            ResponseBody::Http(data) => self.http.continuations.resume(uuid, data),
            ResponseBody::Time(data) => self.time.continuations.resume(uuid, data),
            ResponseBody::Platform(data) => self.platform.continuations.resume(uuid, data),
            ResponseBody::KVRead(data) => self.key_value_read.continuations.resume(uuid, data),
            ResponseBody::KVWrite(data) => self.key_value_write.continuations.resume(uuid, data),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub uuid: Vec<u8>,
    pub body: RequestBody,
}

impl Request {
    pub fn render() -> Self {
        Self {
            uuid: Default::default(),
            body: RequestBody::Render,
        }
    }
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

#[derive(Serialize, Deserialize)]
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
