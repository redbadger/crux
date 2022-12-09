//! TODO mod docs

use crux_core::{
    capability::{CapabilityContext, Operation},
    Capability,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum KeyValueOperation {
    Read(String),
    Write(String, Vec<u8>),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum KeyValueOutput {
    // TODO: Add support for errors
    Read(Option<Vec<u8>>),
    // TODO: Add support for errors
    Write(bool),
}

impl Operation for KeyValueOperation {
    type Output = KeyValueOutput;
}

pub struct KeyValue<Ev> {
    context: CapabilityContext<KeyValueOperation, Ev>,
}

impl<Ev> KeyValue<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<KeyValueOperation, Ev>) -> Self {
        Self { context }
    }

    pub fn read<F>(&self, key: &str, make_event: F)
    where
        F: Fn(KeyValueOutput) -> Ev + Send + Sync + 'static,
    {
        let ctx = self.context.clone();
        let key = key.to_string();
        self.context.spawn(async move {
            let output = ctx.request_from_shell(KeyValueOperation::Read(key)).await;

            ctx.update_app(make_event(output))
        });
    }

    pub fn write<F>(&self, key: &str, value: Vec<u8>, make_event: F)
    where
        F: Fn(KeyValueOutput) -> Ev + Send + Sync + 'static,
    {
        let ctx = self.context.clone();
        let key = key.to_string();
        self.context.spawn(async move {
            let resp = ctx
                .request_from_shell(KeyValueOperation::Write(key, value))
                .await;

            ctx.update_app(make_event(resp))
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
