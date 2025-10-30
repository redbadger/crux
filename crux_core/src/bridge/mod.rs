mod formats;
mod registry;
mod request_serde;

use facet::Facet;
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::fmt::Debug;
use thiserror::Error;

use crate::{App, Core, core::ResolveError};
pub use formats::{BincodeFfiFormat, JsonFfiFormat};
pub use registry::EffectId;
pub(crate) use registry::ResolveRegistry;
// ResolveByte and Response are public to be accessible from crux_macros
#[doc(hidden)]
pub use request_serde::{ResolveSerialized, Response};

/// A serialization format for the bridge FFI.
///
/// **Note**: While you can implement your own format for use with the [`BridgeWithSerializer`],
/// the type generation system doesn't yet support automatically generating the shell-side support
/// for different formats, and you'll need to bring your own solution for this.
pub trait FfiFormat: Debug + 'static {
    type Error: std::error::Error;

    /// Serialize an instance of `T` into the provided growable byte buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    fn serialize<T: Serialize>(buffer: &mut Vec<u8>, value: &T) -> Result<(), Self::Error>;

    /// Deserialize an instance of `T` from the provided byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    fn deserialize<'de, T: Deserialize<'de>>(bytes: &'de [u8]) -> Result<T, Self::Error>;
}

/// Request for a side-effect passed from the Core to the Shell. The `EffectId` links
/// the `Request` with the corresponding call to [`Core::resolve`] to pass the data back
/// to the [`App::update`] function (wrapped in the event provided to the capability originating the effect).
// used in docs/internals/bridge.md
// ANCHOR: request
#[derive(Facet, Debug, Serialize, Deserialize)]
pub struct Request<Eff>
where
    Eff: Serialize,
{
    pub id: EffectId,
    pub effect: Eff,
}
// ANCHOR_END: request

/// Bridge is a core wrapper presenting the same interface as the [`Core`] but in a
/// serialized form, using bincode as the serialization format.
pub struct Bridge<A, T = BincodeFfiFormat>
where
    A: App,
    T: FfiFormat,
{
    inner: BridgeWithSerializer<A, T>,
}

#[derive(Debug, Error)]
pub enum BridgeError<T: FfiFormat> {
    #[error("could not deserialize event: {0}")]
    DeserializeEvent(T::Error),
    #[error("could not deserialize provided effect output: {0}")]
    DeserializeOutput(T::Error),
    #[error("effect output type mismatch: expected {expected:?}, found {found:?}")]
    OutputTypeMismatch { expected: TypeId, found: TypeId },
    #[error("could not process response: {0}")]
    ProcessResponse(#[from] ResolveError),
    #[error("could not serialize effect requests: {0}")]
    SerializeRequests(T::Error),
    #[error("could not serialize view model: {0}")]
    SerializeView(T::Error),
}

impl<A, T> Bridge<A, T>
where
    A: App,
    T: FfiFormat,
{
    /// Create a new Bridge using the provided `core`.
    pub fn new(core: Core<A>) -> Self {
        Self {
            inner: BridgeWithSerializer::new(core),
        }
    }

    /// Receive an event from the shell.
    ///
    /// The `event` is serialized and will be deserialized by the core before it's passed
    /// to your app.
    ///
    /// # Errors
    ///
    /// Returns an error if the event could not be deserialized.
    pub fn process_event(&self, event: &[u8]) -> Result<Vec<u8>, BridgeError<T>>
    where
        A::Event: for<'a> Deserialize<'a>,
        A::Effect: crate::core::EffectFFI,
    {
        let mut return_buffer = vec![];

        self.inner.process_event(event, &mut return_buffer)?;

        Ok(return_buffer)
    }

    /// Receive an event.
    ///
    /// # Errors
    ///
    /// Returns an error if the effects could not be serialized.
    pub fn process_event_typed(&self, event: A::Event) -> Result<Vec<u8>, BridgeError<T>>
    where
        A::Effect: crate::core::EffectFFI,
    {
        let mut return_buffer = vec![];

        self.inner.process_event_typed(event, &mut return_buffer)?;

        Ok(return_buffer)
    }

    /// Receive a response to a capability request from the shell.
    ///
    /// The `output` is serialized capability output. It will be deserialized by the core.
    ///
    /// # Errors
    ///
    /// Returns an error if the response could not be deserialized.
    ///
    /// # Panics
    ///
    /// The `id` MUST match the `id` of the effect that triggered it, else the core will panic.
    // used in docs/internals/bridge.md
    // ANCHOR: handle_response_sig
    pub fn handle_response(&self, id: u32, output: &[u8]) -> Result<Vec<u8>, BridgeError<T>>
    // ANCHOR_END: handle_response_sig
    where
        A::Event: for<'a> Deserialize<'a>,
        A::Effect: crate::core::EffectFFI,
    {
        let mut return_buffer = vec![];

        self.inner.handle_response(id, output, &mut return_buffer)?;

        Ok(return_buffer)
    }

    /// Receive a typed response to a capability request from the shell.
    ///
    /// # Errors
    ///
    /// Returns an error if the response type does not match the expected output type.
    pub fn handle_response_typed<R>(&self, id: u32, output: R) -> Result<Vec<u8>, BridgeError<T>>
    where
        A::Effect: crate::core::EffectFFI,
        R: 'static,
    {
        let mut return_buffer = vec![];

        self.inner
            .handle_response_typed(id, output, &mut return_buffer)?;

        Ok(return_buffer)
    }

    /// Get the current state of the app's view model (serialized).
    ///
    /// # Errors
    ///
    /// Returns an error if the view model could not be serialized.
    pub fn view(&self) -> Result<Vec<u8>, BridgeError<T>>
    where
        A::ViewModel: Serialize,
    {
        let mut return_buffer = vec![];

        self.inner.view(&mut return_buffer)?;

        Ok(return_buffer)
    }
}

/// A bridge with a user supplied serializer
///
/// This is exactly the same as [`Bridge`], except instead of using the default
/// bincode serialization, you can provide your own [`Serializer`](::serde::ser::Serializer).
///
/// **Warning**: the support for custom serialization is **experimental** and
/// does not have a corresponding type generation support - you will need
/// to write deserialization code on the shell side yourself, or generate
/// it using separate tooling.
// used in docs/internals/bridge.md
// ANCHOR: bridge_with_serializer
pub struct BridgeWithSerializer<A, T = BincodeFfiFormat>
where
    A: App,
    T: FfiFormat,
{
    core: Core<A>,
    registry: ResolveRegistry<T>,
}
// ANCHOR_END: bridge_with_serializer

impl<A, T> BridgeWithSerializer<A, T>
where
    A: App,
    T: FfiFormat,
{
    pub fn new(core: Core<A>) -> Self {
        Self {
            core,
            registry: ResolveRegistry::default(),
        }
    }

    /// Receive an event from the shell.
    ///
    /// The `event` is serialized and will be deserialized by the core before it's passed
    /// to your app.
    ///
    /// # Errors
    ///
    /// Returns an error if the event could not be deserialized.
    pub fn process_event<'a>(
        &self,
        event: &'a [u8],
        requests_out: &mut Vec<u8>,
    ) -> Result<(), BridgeError<T>>
    where
        A::Event: Deserialize<'a>,
        A::Effect: crate::core::EffectFFI,
    {
        self.process(None, event, requests_out)
    }

    /// Receive an event.
    ///
    /// # Errors
    ///
    /// Returns an error if the effects could not be serialized.
    pub fn process_event_typed(
        &self,
        event: A::Event,
        requests_out: &mut Vec<u8>,
    ) -> Result<(), BridgeError<T>>
    where
        A::Effect: crate::core::EffectFFI,
    {
        let effects = self.core.process_event(event);
        self.process_effects(effects, requests_out)
    }

    /// Receive a response to a capability request from the shell.
    ///
    /// The `output` is serialized capability output. It will be deserialized by the core.
    ///
    /// # Errors
    ///
    /// Returns an error if the response could not be deserialized.
    ///
    /// # Panics
    ///
    /// The `id` MUST match the `id` of the effect that triggered it, else the core will panic.
    pub fn handle_response<'a>(
        &self,
        id: u32,
        response: &'a [u8],
        requests_out: &mut Vec<u8>,
    ) -> Result<(), BridgeError<T>>
    where
        A::Event: Deserialize<'a>,
        A::Effect: crate::core::EffectFFI,
    {
        self.process(Some(EffectId(id)), response, requests_out)
    }

    /// Receive a typed response to a capability request from the shell.
    ///
    /// # Errors
    ///
    /// Returns an error if the response type does not match the expected output type.
    ///
    /// # Panics
    ///
    /// The `id` MUST match the `id` of the effect that triggered it, else the core will panic.
    pub fn handle_response_typed<R>(
        &self,
        id: u32,
        response: R,
        requests_out: &mut Vec<u8>,
    ) -> Result<(), BridgeError<T>>
    where
        A::Effect: crate::core::EffectFFI,
        R: 'static,
    {
        let effects = {
            self.registry
                .resume(EffectId(id), Response::Value(Box::new(response)))?;

            self.core.process()
        };

        self.process_effects(effects, requests_out)
    }

    fn process<'a>(
        &self,
        id: Option<EffectId>,
        data: &'a [u8],
        requests_out: &mut Vec<u8>,
    ) -> Result<(), BridgeError<T>>
    where
        A::Event: Deserialize<'a>,
        A::Effect: crate::core::EffectFFI,
    {
        let effects = match id {
            None => {
                let shell_event = T::deserialize(data).map_err(BridgeError::DeserializeEvent)?;

                self.core.process_event(shell_event)
            }
            Some(id) => {
                self.registry.resume(id, Response::Bytes(data))?;

                self.core.process()
            }
        };

        self.process_effects(effects, requests_out)
    }

    fn process_effects(
        &self,
        effects: Vec<A::Effect>,
        requests_out: &mut Vec<u8>,
    ) -> Result<(), BridgeError<T>>
    where
        A::Effect: crate::core::EffectFFI,
    {
        let requests: Vec<_> = effects
            .into_iter()
            .map(|eff| self.registry.register(eff))
            .collect();

        T::serialize(requests_out, &requests).map_err(BridgeError::SerializeRequests)?;

        Ok(())
    }

    /// Get the current state of the app's view model (serialized).
    ///
    /// # Errors
    ///
    /// Returns an error if the view model could not be serialized.
    pub fn view(&self, view_out: &mut Vec<u8>) -> Result<(), BridgeError<T>>
    where
        A::ViewModel: Serialize,
    {
        T::serialize(view_out, &self.core.view()).map_err(BridgeError::SerializeView)
    }
}
