//! TODO mod docs

use crux_core::{Capability, Command};
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

pub struct KeyValue<Ef> {
    effect: Box<dyn Fn(KeyValueRequest) -> Ef + Sync>,
}

impl<Ef> KeyValue<Ef> {
    pub fn new<MakeEffect>(effect: MakeEffect) -> Self
    where
        MakeEffect: Fn(KeyValueRequest) -> Ef + Sync + 'static,
    {
        Self {
            effect: Box::new(effect),
        }
    }

    pub fn read<Ev, F>(&self, key: &str, callback: F) -> Command<Ef, Ev>
    where
        Ev: 'static,
        F: Fn(KeyValueResponse) -> Ev + Send + Sync + 'static,
    {
        Command::new(
            (self.effect)(KeyValueRequest::Read(key.to_string())),
            callback,
        )
    }

    pub fn write<Ev, F>(&self, key: &str, value: Vec<u8>, callback: F) -> Command<Ef, Ev>
    where
        Ev: 'static,
        F: Fn(KeyValueResponse) -> Ev + Send + Sync + 'static,
    {
        Command::new(
            (self.effect)(KeyValueRequest::Write(key.to_string(), value)),
            callback,
        )
    }
}

impl<Ef> Capability for KeyValue<Ef> where Ef: Clone {}
