use crate::{Continuations, Request, RequestBody};

pub struct KeyValueRead<'c, Msg> {
    continuations: &'c Continuations<Msg>,
}

impl<'c, Msg> KeyValueRead<'c, Msg> {
    pub fn new(continuations: &'c Continuations<Msg>) -> Self {
        Self { continuations }
    }

    pub fn read<F>(&self, key: String, msg: F) -> Request
    where
        F: FnOnce(Option<Vec<u8>>) -> Msg + Sync + Send + 'static,
    {
        let body = RequestBody::KVRead(key);
        self.continuations
            .key_value_read
            .write()
            .unwrap()
            .pause(body, msg)
    }
}

pub struct KeyValueWrite<'c, Msg> {
    continuations: &'c Continuations<Msg>,
}

impl<'c, Msg> KeyValueWrite<'c, Msg> {
    pub fn new(continuations: &'c Continuations<Msg>) -> Self {
        Self { continuations }
    }

    pub fn write<F>(&self, key: String, value: Vec<u8>, msg: F) -> Request
    where
        F: FnOnce(bool) -> Msg + Sync + Send + 'static,
    {
        let body = RequestBody::KVWrite(key, value);
        self.continuations
            .key_value_write
            .write()
            .unwrap()
            .pause(body, msg)
    }
}
