//! TODO mod docs

use crate::{Capability, Command};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub enum Request {
    Read(String),
    Write(String, Vec<u8>),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Response {
    Read(Option<Vec<u8>>),
    Write(bool),
}

pub struct KeyValue<Ef> {
    effect: Box<dyn Fn(Request) -> Ef + Sync>,
}

impl<Ef> KeyValue<Ef> {
    pub fn new<MakeEffect>(effect: MakeEffect) -> Self
    where
        MakeEffect: Fn(Request) -> Ef + Sync + 'static,
    {
        Self {
            effect: Box::new(effect),
        }
    }

    pub fn read<Ev, F>(&self, key: &str, callback: F) -> Command<Ef, Ev>
    where
        Ev: 'static,
        F: Fn(Response) -> Ev + Send + Sync + 'static,
    {
        Command::new((self.effect)(Request::Read(key.to_string())), callback)
    }

    pub fn write<Ev, F>(&self, key: &str, value: Vec<u8>, callback: F) -> Command<Ef, Ev>
    where
        Ev: 'static,
        F: Fn(Response) -> Ev + Send + Sync + 'static,
    {
        Command::new(
            (self.effect)(Request::Write(key.to_string(), value)),
            callback,
        )
    }
}

impl<Ef> Capability for KeyValue<Ef> where Ef: Clone {}
