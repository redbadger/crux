//! TODO mod docs

use serde::Serialize;

use crate::Command;

#[derive(Serialize)]
pub enum Request {
    Read(String),
    Write(String, Vec<u8>),
}

pub struct KeyValue<MakeEffect, Ef>
where
    MakeEffect: Fn(Request) -> Ef,
{
    effect: MakeEffect,
}

impl<MakeEffect, Ef> KeyValue<MakeEffect, Ef>
where
    MakeEffect: Fn(Request) -> Ef,
{
    pub fn new(effect: MakeEffect) -> Self {
        Self { effect }
    }

    pub fn read<Ev, F>(&self, key: &str, callback: F) -> Command<Ef, Ev>
    where
        Ev: 'static,
        F: Fn(Option<Vec<u8>>) -> Ev + 'static,
    {
        Command::new((self.effect)(Request::Read(key.to_string())), callback)
    }

    pub fn write<Ev, F>(&self, key: &str, value: Vec<u8>, callback: F) -> Command<Ef, Ev>
    where
        Ev: 'static,
        F: Fn(bool) -> Ev + 'static,
    {
        Command::new(
            (self.effect)(Request::Write(key.to_string(), value)),
            callback,
        )
    }
}
