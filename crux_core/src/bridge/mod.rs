mod formats;
#[cfg(feature = "native_bridge")]
mod native;
#[cfg(feature = "native_bridge")]
mod native_registry;
mod registry;
#[cfg(feature = "native_bridge")]
mod request_native;
mod request_serde;

use facet::Facet;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use thiserror::Error;

/// A zero-sized output type for operations that don't return meaningful data.
///
/// This is used instead of `()` for UniFFI compatibility, as `()` doesn't
/// implement the required UniFFI traits (`Lower`, `Lift`).
///
/// Use this as `Operation::Output` for fire-and-forget operations like `Render`.
#[derive(Facet, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "native_bridge", derive(uniffi::Record))]
pub struct UnitOutput;

use crate::{App, Core, core::ResolveError};

/// Default initial capacity for the resolve registry slab.
///
/// This balances memory usage against allocation overhead. The slab
/// grows dynamically if more concurrent effects are needed.
pub(crate) const DEFAULT_REGISTRY_CAPACITY: usize = 1024;

pub use formats::{BincodeFfiFormat, JsonFfiFormat};
pub use registry::EffectId;
pub(crate) use registry::ResolveRegistry;
// ResolveByte is public to be accessible from crux_macros
#[doc(hidden)]
pub use request_serde::ResolveSerialized;

#[cfg(feature = "native_bridge")]
pub use native::NativeBridge;
#[cfg(feature = "native_bridge")]
pub use request_native::ResolveNative;

/// Error type for the native typed bridge.
///
/// Uses `String` fields (not `&'static str` or foreign error types) for
/// UniFFI compatibility — UniFFI errors need owned types.
#[cfg(feature = "native_bridge")]
#[derive(Debug, Error, uniffi::Error)]
pub enum NativeBridgeError {
    #[error("effect output variant mismatch: expected {expected}")]
    OutputMismatch { expected: String },
    #[error("effect id {effect_id} not found")]
    EffectNotFound { effect_id: u32 },
    #[error("stream resolve finished")]
    ResolveFinished,
    #[error("attempted to resolve fire-and-forget effect")]
    ResolveNever,
}

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
pub struct Bridge<A, F = BincodeFfiFormat>
where
    A: App,
    F: FfiFormat,
{
    core: Core<A>,
    registry: ResolveRegistry<F>,
}

#[derive(Debug, Error)]
pub enum BridgeError<F: FfiFormat = BincodeFfiFormat> {
    #[error("could not deserialize event: {0}")]
    DeserializeEvent(F::Error),
    #[error("could not deserialize provided effect output: {0}")]
    DeserializeOutput(F::Error),
    #[error("could not process response: {0}")]
    ProcessResponse(#[from] ResolveError),
    #[error("could not serialize effect requests: {0}")]
    SerializeRequests(F::Error),
    #[error("could not serialize view model: {0}")]
    SerializeView(F::Error),
}

impl<A, Format> Bridge<A, Format>
where
    A: App,
    Format: FfiFormat,
{
    /// Create a new Bridge using the provided `core`.
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
    #[deprecated(
        since = "0.17.0",
        note = "Bridge API returning vectors has been deprecated. Please use the 'update' method."
    )]
    pub fn process_event(&self, event: &[u8]) -> Result<Vec<u8>, BridgeError<Format>>
    where
        A::Event: for<'a> Deserialize<'a>,
        A::Effect: crate::core::EffectFFI,
    {
        let mut return_buffer = vec![];

        self.update(event, &mut return_buffer)?;

        Ok(return_buffer)
    }

    /// Send an event from the shell.
    ///
    /// The `event` is serialized and will be deserialized by the core before it's passed
    /// to your app.
    ///
    /// # Errors
    ///
    /// Returns an error if the event could not be deserialized.
    pub fn update<'a>(
        &self,
        event: &'a [u8],
        requests_out: &mut Vec<u8>,
    ) -> Result<(), BridgeError<Format>>
    where
        A::Event: Deserialize<'a>,
        A::Effect: crate::core::EffectFFI,
    {
        self.process(None, event, requests_out)
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
    #[deprecated(
        since = "0.17.0",
        note = "Bridge API returning vectors has been deprecated. Please use the 'resolve' method."
    )]
    pub fn handle_response(&self, id: u32, output: &[u8]) -> Result<Vec<u8>, BridgeError<Format>>
    // ANCHOR_END: handle_response_sig
    where
        A::Event: for<'a> Deserialize<'a>,
        A::Effect: crate::core::EffectFFI,
    {
        let mut return_buffer = vec![];

        self.resolve(EffectId(id), output, &mut return_buffer)?;

        Ok(return_buffer)
    }

    /// Provide a response to a capability request to resolve it and continue the corresponding command.
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
    pub fn resolve<'a>(
        &self,
        id: EffectId,
        response: &'a [u8],
        requests_out: &mut Vec<u8>,
    ) -> Result<(), BridgeError<Format>>
    where
        A::Event: Deserialize<'a>,
        A::Effect: crate::core::EffectFFI,
    {
        self.process(Some(id), response, requests_out)
    }

    fn process<'a>(
        &self,
        id: Option<EffectId>,
        data: &'a [u8],
        requests_out: &mut Vec<u8>,
    ) -> Result<(), BridgeError<Format>>
    where
        A::Event: Deserialize<'a>,
        A::Effect: crate::core::EffectFFI,
    {
        let effects = match id {
            None => {
                let shell_event =
                    Format::deserialize(data).map_err(BridgeError::DeserializeEvent)?;

                self.core.process_event(shell_event)
            }
            Some(id) => {
                self.registry.resume(id, data)?;

                self.core.process()
            }
        };

        self.process_effects(effects, requests_out)
    }

    fn process_effects(
        &self,
        effects: Vec<A::Effect>,
        requests_out: &mut Vec<u8>,
    ) -> Result<(), BridgeError<Format>>
    where
        A::Effect: crate::core::EffectFFI,
    {
        let requests: Vec<_> = effects
            .into_iter()
            .map(|eff| self.registry.register(eff))
            .collect();

        Format::serialize(requests_out, &requests).map_err(BridgeError::SerializeRequests)?;

        Ok(())
    }

    /// Get the current state of the app's view model (serialized).
    ///
    /// # Errors
    ///
    /// Returns an error if the view model could not be serialized.
    pub fn view(&self, view_out: &mut Vec<u8>) -> Result<(), BridgeError<Format>>
    where
        A::ViewModel: Serialize,
    {
        Format::serialize(view_out, &self.core.view()).map_err(BridgeError::SerializeView)
    }
}
