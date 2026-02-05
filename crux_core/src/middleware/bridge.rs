use std::{marker::PhantomData, sync::Arc};

use serde::{Deserialize, Serialize};

use super::Layer;
use crate::{
    EffectFFI,
    bridge::{BridgeError, EffectId, FfiFormat, ResolveRegistry},
};

#[doc(hidden)]
pub type EffectCallback<Format> =
    dyn Fn(Result<Vec<u8>, BridgeError<Format>>) + Send + Sync + 'static;

/// FFI Bridge with support for wrapping a middleware stack
pub struct Bridge<Next, Format>
where
    Next: Layer,
    Format: FfiFormat,
{
    next: Next,
    effect_callback: Arc<EffectCallback<Format>>,
    registry: Arc<ResolveRegistry<Format>>,
    format: PhantomData<Format>,
}

impl<Next, Format> Bridge<Next, Format>
where
    Next: Layer,
    Next::Event: for<'a> Deserialize<'a>,
    Next::Effect: EffectFFI,
    Format: FfiFormat,
{
    /// Typically, you would would use [`Layer::bridge`] to construct a `Bridge` instance
    pub fn new<F>(next: Next, effect_callback: F) -> Self
    where
        F: Fn(Result<Vec<u8>, BridgeError<Format>>) + Send + Sync + 'static,
    {
        Self {
            next,
            effect_callback: Arc::new(effect_callback),
            registry: Arc::new(ResolveRegistry::default()),
            format: PhantomData,
        }
    }

    /// Send a serialized event to the core
    ///
    /// # Errors
    ///
    /// Returns an [`BridgeError`] when any of the (de)serialization fails
    pub fn update(
        &self,
        event_bytes: &[u8],
        requests_out: &mut Vec<u8>,
    ) -> Result<(), BridgeError<Format>> {
        self.process(None, event_bytes, requests_out)
    }

    /// Resolve a requested effect, providing the output to the core
    ///
    /// # Errors
    ///
    /// Returns a [`BridgeError`] when the effect fails to resolve, or any of the (de)serialization fails.
    pub fn resolve(
        &self,
        id: EffectId,
        output: &[u8],
        requests_out: &mut Vec<u8>,
    ) -> Result<(), BridgeError<Format>> {
        self.process(Some(id), output, requests_out)
    }

    fn process(
        &self,
        id: Option<EffectId>,
        event_or_output: &[u8],
        requests_out: &mut Vec<u8>,
    ) -> Result<(), BridgeError<Format>> {
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
                    Format::serialize(&mut requests_bytes, &requests)
                        .map_err(BridgeError::SerializeRequests)
                };

                shell_callback(result.map(|()| requests_bytes));
            }
        };

        let effects = match id {
            None => {
                let shell_event =
                    Format::deserialize(event_or_output).map_err(BridgeError::DeserializeEvent)?;

                self.next.update(shell_event, effect_callback)
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

        Format::serialize(requests_out, &requests).map_err(BridgeError::SerializeRequests)?;

        Ok(())
    }

    /// Get the latest view model
    ///
    /// # Errors
    ///
    /// Returns an [`BridgeError`] when any of the (de)serialization fails
    pub fn view(&self, view_out: &mut Vec<u8>) -> Result<(), BridgeError<Format>>
    where
        Next::ViewModel: Serialize,
    {
        Format::serialize(view_out, &self.next.view()).map_err(BridgeError::SerializeView)
    }
}
