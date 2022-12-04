//! TODO mod docs

use crux_core::{capability::CapabilityContext, Capability};
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

impl crux_core::Effect for KeyValueRequest {
    type Response = KeyValueResponse;
}

pub struct KeyValue<Ev> {
    context: CapabilityContext<KeyValueRequest, Ev>,
}

impl<Ev> KeyValue<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<KeyValueRequest, Ev>) -> Self {
        Self { context }
    }

    pub fn read<F>(&self, key: &str, callback: F)
    where
        F: Fn(KeyValueResponse) -> Ev + Send + Sync + 'static,
    {
        let ctx = self.context.clone();
        let key = key.to_string();
        self.context.spawn(async move {
            let resp = ctx.effect(KeyValueRequest::Read(key)).await;

            ctx.send_event(callback(resp))
        });
    }

    pub fn write<F>(&self, key: &str, value: Vec<u8>, callback: F)
    where
        F: Fn(KeyValueResponse) -> Ev + Send + Sync + 'static,
    {
        let ctx = self.context.clone();
        let key = key.to_string();
        self.context.spawn(async move {
            let resp = ctx.effect(KeyValueRequest::Write(key, value)).await;

            ctx.send_event(callback(resp))
        });
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
        KeyValue::new(self.context.map_event(f))
    }
}
