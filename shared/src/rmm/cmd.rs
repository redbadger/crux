use super::{http::Http, key_value::KeyValue, time::Time};

// TODO consider whether these fields should be public
pub struct Cmd<Msg> {
    pub http: Http<Msg>,
    pub time: Time<Msg>,
    pub key_value: KeyValue<Msg>,
}

impl<Msg> Default for Cmd<Msg> {
    fn default() -> Self {
        Self {
            http: Http::default(),
            time: Time::default(),
            key_value: KeyValue::default(),
        }
    }
}

impl<Msg> Cmd<Msg> {
    pub fn http_get<F>(&self, url: String, msg: F) -> Request
    where
        F: Send + Sync + 'static + FnOnce(Vec<u8>) -> Msg,
    {
        self.http.get(url, msg)
    }

    pub fn time<F>(&self, msg: F) -> Request
    where
        F: Send + Sync + 'static + FnOnce(String) -> Msg,
    {
        self.time.get(msg)
    }

    pub fn kv_write<F>(&self, key: String, bytes: Vec<u8>, msg: F) -> Request
    where
        F: Send + Sync + 'static + FnOnce(bool) -> Msg,
    {
        self.key_value.write(key, bytes, msg)
    }

    pub fn kv_read<F>(&self, key: String, msg: F) -> Request
    where
        F: Send + Sync + 'static + FnOnce(Option<Vec<u8>>) -> Msg,
    {
        self.key_value.read(key, msg)
    }

    pub fn render(&self) -> Request {
        Request::Render
    }
}

pub enum Request {
    Http {
        uuid: Vec<u8>,
        url: String,
    },
    Time {
        uuid: Vec<u8>,
    },
    KVRead {
        uuid: Vec<u8>,
        key: String,
    },
    KVWrite {
        uuid: Vec<u8>,
        key: String,
        bytes: Vec<u8>,
    },
    Render,
}

pub enum Response {
    Http {
        uuid: Vec<u8>,
        bytes: Vec<u8>,
    },
    Time {
        uuid: Vec<u8>,
        iso_time: String,
    },
    KVRead {
        uuid: Vec<u8>,
        bytes: Option<Vec<u8>>,
    },
    KVWrite {
        uuid: Vec<u8>,
        success: bool,
    },
}
