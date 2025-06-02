use std::sync::Arc;

use bincode::{de::read::SliceReader, DefaultOptions, Options};
use erased_serde::Serialize;
use serde::Deserialize;

use crate::{
    bridge::{BridgeError, EffectId, ResolveRegistry},
    Effect,
};

use super::Layer;

/// A serialization format for the bridge FFI
pub trait FfiFormat {
    type Serializer<'s>: 's;

    type Deserializer<'d>: 'd;

    fn serializer<'b, 's>(buffer: &'b mut [u8]) -> Self::Serializer<'s>
    where
        'b: 's;

    fn deserializer<'b, 'd>(bytes: &'b [u8]) -> Self::Deserializer<'d>
    where
        'b: 'd;
}

// TODO: move out!

struct BincodeFfiFormat;

impl BincodeFfiFormat {
    fn bincode_options() -> impl bincode::Options + Copy {
        DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes()
    }
}

impl BincodeFfiFormat {
    // TODO: turn into trait impl
    fn deserializer<'b>(
        bytes: &'b [u8],
    ) -> bincode::Deserializer<SliceReader<'b>, impl bincode::Options + Copy> {
        bincode::Deserializer::from_slice(bytes, Self::bincode_options())
    }

    fn serializer<'b>(
        buffer: &'b mut Vec<u8>,
    ) -> bincode::Serializer<&'b mut Vec<u8>, impl bincode::Options + Copy> {
        bincode::Serializer::new(buffer, Self::bincode_options())
    }
}

/// FFI Bridge with support for wrapping a middlware stack
pub struct Bridge<Next>
where
    Next: Layer,
    // Fmt: FfiFormat,
{
    next: Next,
    effect_callback: Arc<dyn Fn(Result<Vec<u8>, BridgeError>) + Send + Sync + 'static>,
    registry: Arc<ResolveRegistry>,
    // format: Fmt,
}

impl<Next> Bridge<Next>
where
    Next: Layer,
    Next::Event: for<'a> Deserialize<'a>,
    Next::Effect: Effect,
    // Fmt: Fmt,
{
    pub fn new<F>(next: Next, effect_callback: F) -> Self
    where
        F: Fn(Result<Vec<u8>, BridgeError>) + Send + Sync + 'static,
    {
        Self {
            next,
            effect_callback: Arc::new(effect_callback),
            registry: Arc::new(ResolveRegistry::default()),
            // format: PhantomData,
        }
    }

    pub fn process_event<'b>(&self, event_bytes: &'b [u8]) -> Result<Vec<u8>, BridgeError> {
        let mut requests_bytes = vec![];

        let result = {
            // scope lifetime of the (de)serializers
            let mut event_de = BincodeFfiFormat::deserializer(event_bytes);
            let mut erased_event_de = <dyn erased_serde::Deserializer>::erase(&mut event_de);

            let mut request_se = BincodeFfiFormat::serializer(&mut requests_bytes);
            let mut erased_request_se = <dyn erased_serde::Serializer>::erase(&mut request_se);

            self.process(None, &mut erased_event_de, &mut erased_request_se)
        };

        result.map(|()| requests_bytes)
    }

    pub fn resolve(&self, id: EffectId, output: &[u8]) -> Result<Vec<u8>, BridgeError> {
        let mut requests_bytes = vec![];

        let result = {
            let mut output_de = BincodeFfiFormat::deserializer(output);
            let mut erased_output_de = <dyn erased_serde::Deserializer>::erase(&mut output_de);

            let mut request_se = BincodeFfiFormat::serializer(&mut requests_bytes);
            let mut erased_request_se = <dyn erased_serde::Serializer>::erase(&mut request_se);

            self.process(Some(id), &mut erased_output_de, &mut erased_request_se)
        };

        result.map(|()| requests_bytes)
    }

    fn process(
        &self,
        id: Option<EffectId>,
        event_or_output: &mut dyn erased_serde::Deserializer,
        requests_out: &mut dyn erased_serde::Serializer,
    ) -> Result<(), BridgeError> {
        let effect_callback = {
            let shell_callback = self.effect_callback.clone();
            let registry = self.registry.clone();

            move |effects: Vec<Next::Effect>| {
                let requests: Vec<_> = effects
                    .into_iter()
                    .map(|eff| registry.register(eff))
                    .collect();
                let mut requests_bytes = vec![];

                let result = {
                    let mut requests_se = BincodeFfiFormat::serializer(&mut requests_bytes);
                    let mut erased_request_se =
                        <dyn erased_serde::Serializer>::erase(&mut requests_se);

                    requests
                        .erased_serialize(&mut erased_request_se)
                        .map_err(BridgeError::SerializeRequests)
                };

                shell_callback(result.map(|()| requests_bytes));
            }
        };

        let effects = match id {
            None => {
                let shell_event = erased_serde::deserialize(event_or_output)
                    .map_err(BridgeError::DeserializeEvent)?;

                self.next.process_event(shell_event, effect_callback)
            }
            Some(id) => {
                self.registry.resume(id, event_or_output)?;

                self.next.process_tasks(effect_callback)
            }
        };

        let requests: Vec<_> = effects
            .into_iter()
            .map(|eff| self.registry.register(eff))
            .collect();

        requests
            .erased_serialize(requests_out)
            .map_err(BridgeError::SerializeRequests)?;

        Ok(())
    }

    pub fn view(&self) -> Result<Vec<u8>, BridgeError> {
        todo!()
    }
}
