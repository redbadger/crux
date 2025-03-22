mod nacre;
mod registry;
mod request_serde;
mod shell;

use std::sync::Arc;

use bincode::{DefaultOptions, Options};
use either::Either;
use erased_serde::{Error as SerdeError, Serialize as _};
use nacre::ShellEffects;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    core::{Middleware, ResolveError},
    App, Effect, NacreEffect,
};
use registry::{EffectId, ResolveRegistry};
// ResolveByte is public to be accessible from crux_macros
pub use nacre::{Nacre, NacreBridge};
#[doc(hidden)]
pub use request_serde::ResolveSerialized;
pub use shell::Shell;

/// Request for a side-effect passed from the Core to the Shell. The `EffectId` links
/// the `Request` with the corresponding call to [`Core::resolve`] to pass the data back
/// to the [`App::update`] function (wrapped in the event provided to the capability originating the effect).
// used in docs/internals/bridge.md
// ANCHOR: request
#[derive(Debug, Serialize, Deserialize)]
pub struct Request<Eff>
where
    Eff: Serialize,
{
    pub id: EffectId,
    pub effect: Eff,
}
// ANCHOR_END: request

pub type RequestFfi<A> = Request<<<A as App>::Effect as Effect>::Ffi>;

pub trait NacreTrait<A: App>
where
    A::Effect: NacreEffect,
{
    fn register_callback(
        &self,
        receiver: async_std::channel::Receiver<ShellEffects<A>>,
        cb: impl Fn(ShellEffects<A>) -> Vec<u8> + Send + 'static,
    );
}

/// Bridge is a core wrapper presenting the same interface as the [`Core`] but in a
/// serialized form, using bincode as the serialization format.
pub struct Bridge<Core>
where
    Core: Middleware,
{
    inner: Arc<BridgeWithSerializer<Core>>,
}

impl<Core> Bridge<Core>
where
    Core: NacreTrait<Core::App> + Middleware + Send + Sync + 'static,
    <<Core as Middleware>::App as App>::Effect: NacreEffect,
    <<Core as Middleware>::App as App>::Effect:
        From<<<<Core as Middleware>::App as App>::Effect as NacreEffect>::ShellEffect>,
{
    pub fn from_nacre(
        nacre: Core,
        receiver: async_std::channel::Receiver<ShellEffects<Core::App>>,
    ) -> Self {
        Self {
            inner: BridgeWithSerializer::from_nacre(nacre, receiver),
        }
    }
}

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("could not deserialize event: {0}")]
    DeserializeEvent(SerdeError),
    #[error("could not deserialize provided effect output: {0}")]
    DeserializeOutput(SerdeError),
    #[error("could not process response: {0}")]
    ProcessResponse(#[from] ResolveError),
    #[error("could not serialize effect requests: {0}")]
    SerializeRequests(SerdeError),
    #[error("could not serialize view model: {0}")]
    SerializeView(SerdeError),
}

impl<Core> Bridge<Core>
where
    Core: Middleware,
{
    /// Create a new Bridge using the provided `core`.
    pub fn new(core: Core) -> Self {
        Self {
            inner: Arc::new(BridgeWithSerializer::new(core)),
        }
    }

    /// Receive an event from the shell.
    ///
    /// The `event` is serialized and will be deserialized by the core before it's passed
    /// to your app.
    pub fn process_event(&self, event: &[u8]) -> Result<Vec<u8>, BridgeError>
    where
        <Core::App as App>::Event: for<'a> Deserialize<'a>,
    {
        let options = Self::bincode_options();

        let mut deser = bincode::Deserializer::from_slice(event, options);

        let mut return_buffer = vec![];
        let mut ser = bincode::Serializer::new(&mut return_buffer, options);

        self.inner.process_event(&mut deser, &mut ser)?;

        Ok(return_buffer)
    }

    /// Receive a response to a capability request from the shell.
    ///
    /// The `output` is serialized capability output. It will be deserialized by the core.
    /// The `id` MUST match the `id` of the effect that triggered it, else the core will panic.
    // used in docs/internals/bridge.md
    // ANCHOR: handle_response_sig
    pub fn handle_response(&self, id: u32, output: &[u8]) -> Result<Vec<u8>, BridgeError>
    // ANCHOR_END: handle_response_sig
    where
        <Core::App as App>::Event: for<'a> Deserialize<'a>,
    {
        let options = Self::bincode_options();

        let mut deser = bincode::Deserializer::from_slice(output, options);

        let mut return_buffer = vec![];
        let mut ser = bincode::Serializer::new(&mut return_buffer, options);

        self.inner.handle_response(id, &mut deser, &mut ser)?;

        Ok(return_buffer)
    }

    /// Get the current state of the app's view model (serialized).
    pub fn view(&self) -> Result<Vec<u8>, BridgeError> {
        let options = Self::bincode_options();

        let mut return_buffer = vec![];

        self.inner
            .view(&mut bincode::Serializer::new(&mut return_buffer, options))?;

        Ok(return_buffer)
    }

    fn bincode_options() -> impl bincode::Options + Copy {
        DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes()
    }
}

impl<Core> BridgeWithSerializer<Core>
where
    Core: NacreTrait<Core::App> + Middleware + Send + Sync + 'static,
    <<Core as Middleware>::App as App>::Effect: NacreEffect,
    <<Core as Middleware>::App as App>::Effect:
        From<<<<Core as Middleware>::App as App>::Effect as NacreEffect>::ShellEffect>,
{
    pub fn from_nacre(
        nacre: Core,
        receiver: async_std::channel::Receiver<ShellEffects<Core::App>>,
    ) -> Arc<Self> {
        let core = Arc::new(nacre);
        let cloned_core = core.clone();
        let this = Arc::new(Self {
            core,
            registry: Default::default(),
        });
        let cloned = this.clone();
        let callback = move |effects: ShellEffects<Core::App>| {
            let mut return_buffer = vec![];
            let mut ser =
                bincode::Serializer::new(&mut return_buffer, Bridge::<Core>::bincode_options());
            cloned
                .register(
                    effects.into_iter().map(std::convert::Into::into),
                    &mut <dyn erased_serde::Serializer>::erase(&mut ser),
                )
                .unwrap();
            return_buffer
        };
        cloned_core.register_callback(receiver, callback);
        this
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
pub struct BridgeWithSerializer<Core>
where
    Core: Middleware,
{
    core: Arc<Core>,
    registry: ResolveRegistry,
}
// ANCHOR_END: bridge_with_serializer

impl<Core> BridgeWithSerializer<Core>
where
    Core: Middleware,
{
    pub fn new(core: Core) -> Self {
        Self {
            core: Arc::new(core),
            registry: Default::default(),
        }
    }

    /// Receive an event from the shell.
    ///
    /// The `event` is serialized and will be deserialized by the core before it's passed
    /// to your app.
    pub fn process_event<'de, D, S>(&self, event: D, requests_out: S) -> Result<(), BridgeError>
    where
        for<'a> <Core::App as App>::Event: Deserialize<'a>,
        D: ::serde::de::Deserializer<'de> + 'de,
        S: ::serde::ser::Serializer,
    {
        let mut erased_de = <dyn erased_serde::Deserializer>::erase(event);
        self.process(
            None,
            &mut erased_de,
            &mut <dyn erased_serde::Serializer>::erase(requests_out),
        )
    }

    /// Receive a response to a capability request from the shell.
    ///
    /// The `output` is serialized capability output. It will be deserialized by the core.
    /// The `id` MUST match the `id` of the effect that triggered it, else the core will panic.
    pub fn handle_response<'de, D, S>(
        &self,
        id: u32,
        response: D,
        requests_out: S,
    ) -> Result<(), BridgeError>
    where
        for<'a> <Core::App as App>::Event: Deserialize<'a>,
        D: ::serde::de::Deserializer<'de>,
        S: ::serde::ser::Serializer,
    {
        let mut erased_response = <dyn erased_serde::Deserializer>::erase(response);
        self.process(
            Some(EffectId(id)),
            &mut erased_response,
            &mut <dyn erased_serde::Serializer>::erase(requests_out),
        )
    }

    fn process(
        &self,
        id: Option<EffectId>,
        data: &mut dyn erased_serde::Deserializer,
        requests_out: &mut dyn erased_serde::Serializer,
    ) -> Result<(), BridgeError>
    where
        <Core::App as App>::Event: for<'a> Deserialize<'a>,
    {
        let effects = match id {
            None => {
                let shell_event =
                    erased_serde::deserialize(data).map_err(BridgeError::DeserializeEvent)?;

                Either::Left(self.core.process_event(shell_event))
            }
            Some(id) => {
                self.registry.resume(id, data).expect(
                    "Response could not be handled. The request did not expect a response.",
                );

                Either::Right(self.core.process_effects())
            }
        };

        self.register(effects, requests_out)
    }

    fn register(
        &self,
        effects: impl Iterator<Item = <Core::App as App>::Effect>,
        requests_out: &mut dyn erased_serde::Serializer,
    ) -> Result<(), BridgeError> {
        let requests: Vec<_> = effects
            .into_iter()
            .map(|eff| self.registry.register(eff))
            .collect();
        tracing::info!("bridge registered {} effects", requests.len());

        requests
            .erased_serialize(requests_out)
            .map_err(BridgeError::SerializeRequests)?;

        Ok(())
    }

    pub fn inner(&self) -> &Core {
        &self.core
    }

    /// Get the current state of the app's view model (serialized).
    pub fn view<S>(&self, ser: S) -> Result<(), BridgeError>
    where
        S: ::serde::ser::Serializer,
    {
        self.core
            .view()
            .erased_serialize(&mut <dyn erased_serde::Serializer>::erase(ser))
            .map_err(BridgeError::SerializeView)
    }
}
