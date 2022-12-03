//! TODO mod docs

use crux_core::{channels::Sender, Capability, Command};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum KeyValueRequest {
    Read(String),
    Write(String, Vec<u8>),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum KeyValueResponse {
    Read(Option<Vec<u8>>),
    Write(bool),
}

pub struct KeyValue<Ev> {
    sender: Sender<Command<KeyValueRequest, Ev>>,
}

impl<Ev> KeyValue<Ev>
where
    Ev: 'static,
{
    pub fn new(sender: Sender<Command<KeyValueRequest, Ev>>) -> Self {
        Self { sender }
    }

    pub fn read<F>(&self, key: &str, callback: F)
    where
        F: Fn(KeyValueResponse) -> Ev + Send + Sync + 'static,
    {
        self.sender.send(Command::new(
            KeyValueRequest::Read(key.to_string()),
            callback,
        ));
    }

    pub fn write<F>(&self, key: &str, value: Vec<u8>, callback: F)
    where
        F: Fn(KeyValueResponse) -> Ev + Send + Sync + 'static,
    {
        self.sender.send(Command::new(
            KeyValueRequest::Write(key.to_string(), value),
            callback,
        ))
    }
}

impl<Ef> Capability<Ef> for KeyValue<Ef> {
    type MappedSelf<MappedEv> = KeyValue<MappedEv>;

    fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
    where
        F: Fn(NewEvent) -> Ef + Send + Sync + Copy + 'static,
        Ef: 'static,
        NewEvent: 'static,
    {
        KeyValue::new(self.sender.map_event(f))
    }
}
