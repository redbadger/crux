use std::{collections::HashMap, sync::RwLock};

use uuid::Uuid;

pub struct Http<Msg> {
    continuations: RwLock<HashMap<[u8; 16], Box<dyn FnOnce(Vec<u8>) -> Msg + Sync + Send>>>,
}

impl<Msg> Default for Http<Msg> {
    fn default() -> Self {
        Self {
            continuations: RwLock::new(HashMap::new()),
        }
    }
}

impl<Msg> Http<Msg> {
    pub fn get<F>(&self, url: String, msg: F) -> Request
    where
        F: Sync + Send + FnOnce(Vec<u8>) -> Msg + 'static,
    {
        let uuid = *Uuid::new_v4().as_bytes();

        self.continuations
            .write()
            .unwrap()
            .insert(uuid, Box::new(msg));

        Request::Http {
            uuid: uuid.to_vec(),
            url,
        }
    }

    pub fn receive(&self, uuid: &[u8], data: Vec<u8>) -> Msg {
        let mut continuations = self.continuations.write().unwrap();
        let f = continuations.remove(uuid).unwrap();

        f(data)
    }
}

pub struct Time<Msg> {
    continuations: RwLock<HashMap<[u8; 16], Box<dyn FnOnce(String) -> Msg + Sync + Send>>>,
}

impl<Msg> Default for Time<Msg> {
    fn default() -> Self {
        Self {
            continuations: RwLock::new(HashMap::new()),
        }
    }
}

impl<Msg> Time<Msg> {
    pub fn get<F>(&self, msg: F) -> Request
    where
        F: Sync + Send + FnOnce(String) -> Msg + 'static,
    {
        let uuid = *Uuid::new_v4().as_bytes();

        self.continuations
            .write()
            .unwrap()
            .insert(uuid, Box::new(msg));

        Request::Time {
            uuid: uuid.to_vec(),
        }
    }

    pub fn receive(&self, uuid: &[u8], data: String) -> Msg {
        let mut continuations = self.continuations.write().unwrap();
        let f = continuations.remove(uuid).unwrap();

        f(data)
    }
}

pub struct KeyValue<Msg> {
    reads: RwLock<HashMap<[u8; 16], Box<dyn FnOnce(Option<Vec<u8>>) -> Msg + Sync + Send>>>,
    writes: RwLock<HashMap<[u8; 16], Box<dyn FnOnce(bool) -> Msg + Sync + Send>>>,
}

impl<Msg> Default for KeyValue<Msg> {
    fn default() -> Self {
        Self {
            reads: RwLock::new(HashMap::new()),
            writes: RwLock::new(HashMap::new()),
        }
    }
}

impl<Msg> KeyValue<Msg> {
    pub fn write<F>(&self, key: String, bytes: Vec<u8>, msg: F) -> Request
    where
        F: Sync + Send + FnOnce(bool) -> Msg + 'static,
    {
        let uuid = *Uuid::new_v4().as_bytes();

        self.writes.write().unwrap().insert(uuid, Box::new(msg));

        Request::KVWrite {
            uuid: uuid.to_vec(),
            key,
            bytes,
        }
    }

    pub fn written(&self, uuid: &[u8], result: bool) -> Msg {
        let mut writes = self.writes.write().unwrap();
        let f = writes.remove(uuid).unwrap();

        f(result)
    }

    pub fn read<F>(&self, key: String, msg: F) -> Request
    where
        F: Sync + Send + FnOnce(Option<Vec<u8>>) -> Msg + 'static,
    {
        let uuid = *Uuid::new_v4().as_bytes();

        self.reads.write().unwrap().insert(uuid, Box::new(msg));

        Request::KVRead {
            uuid: uuid.to_vec(),
            key,
        }
    }

    pub fn receive_read(&self, uuid: &[u8], bytes: Option<Vec<u8>>) -> Msg {
        let mut reads = self.reads.write().unwrap();
        let f = reads.remove(uuid).unwrap();

        f(bytes)
    }
}

// TODO consider wheteher these fields should be public
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
