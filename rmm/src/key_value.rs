use crate::{continuations::ContinuationStore, Request, RequestBody};

pub struct KeyValueRead<Msg> {
    pub continuations: ContinuationStore<Option<Vec<u8>>, Msg>,
}

impl<Msg> Default for KeyValueRead<Msg> {
    fn default() -> Self {
        Self {
            continuations: Default::default(),
        }
    }
}

impl<Msg> KeyValueRead<Msg> {
    pub fn read<F>(&mut self, key: String, msg: F) -> Request
    where
        F: FnOnce(Option<Vec<u8>>) -> Msg + Sync + Send + 'static,
    {
        let body = RequestBody::KVRead(key);
        self.continuations.pause(body, msg)
    }
}

pub struct KeyValueWrite<Msg> {
    pub continuations: ContinuationStore<bool, Msg>,
}

impl<Msg> Default for KeyValueWrite<Msg> {
    fn default() -> Self {
        Self {
            continuations: Default::default(),
        }
    }
}

impl<Msg> KeyValueWrite<Msg> {
    pub fn write<F>(&mut self, key: String, value: Vec<u8>, msg: F) -> Request
    where
        F: FnOnce(bool) -> Msg + Sync + Send + 'static,
    {
        let body = RequestBody::KVWrite(key, value);
        self.continuations.pause(body, msg)
    }
}
