use super::capability::Capability;
use crate::Request;
use derive_more::Deref;

pub struct KeyValue {
    pub key: String,
    pub value: Vec<u8>,
}
#[derive(Deref)]
pub struct KeyValueRead<Msg>(Capability<Msg, String, Option<Vec<u8>>>);

impl<Msg> Default for KeyValueRead<Msg> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<Msg> KeyValueRead<Msg> {
    pub fn read<F>(&self, key: String, msg: F) -> Request
    where
        F: FnOnce(Option<Vec<u8>>) -> Msg + Sync + Send + 'static,
    {
        Request::KVRead {
            data: self.0.request(key, msg),
        }
    }
}

#[derive(Deref)]
pub struct KeyValueWrite<Msg>(Capability<Msg, KeyValue, bool>);

impl<Msg> Default for KeyValueWrite<Msg> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<Msg> KeyValueWrite<Msg> {
    pub fn write<F>(&self, key: String, value: Vec<u8>, msg: F) -> Request
    where
        F: FnOnce(bool) -> Msg + Sync + Send + 'static,
    {
        Request::KVWrite {
            data: self.0.request(KeyValue { key, value }, msg),
        }
    }
}
