mod registry;
mod request_serde;

use bincode::{DefaultOptions, Options};
use erased_serde::Serialize as _;
use serde::{Deserialize, Serialize};

use crate::Effect;
use crate::{App, Core};
use registry::{EffectId, ResolveRegistry};
// ResolveByte is public to be accessible from crux_macros
#[doc(hidden)]
pub use request_serde::ResolveSerialized;

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

/// Bridge is a core wrapper presenting the same interface as the [`Core`] but in a
/// serialized form, using bincode as the serialization format.
pub struct Bridge<Eff, A>
where
    Eff: Effect,
    A: App,
{
    inner: BridgeWithSerializer<Eff, A>,
}

impl<Eff, A> Bridge<Eff, A>
where
    Eff: Effect + Send + 'static,
    A: App,
{
    /// Create a new Bridge using the provided `core`.
    pub fn new(core: Core<Eff, A>) -> Self {
        Self {
            inner: BridgeWithSerializer::new(core),
        }
    }

    /// Receive an event from the shell.
    ///
    /// The `event` is serialized and will be deserialized by the core before it's passed
    /// to your app.
    pub fn process_event(&self, event: &[u8]) -> Vec<u8>
    where
        A::Event: for<'a> Deserialize<'a>,
    {
        let options = Self::bincode_options();

        let mut deser = bincode::Deserializer::from_slice(event, options);

        let mut return_buffer = vec![];
        let mut ser = bincode::Serializer::new(&mut return_buffer, options);

        self.inner.process_event(&mut deser, &mut ser);

        return_buffer
    }

    /// Receive a response to a capability request from the shell.
    ///
    /// The `output` is serialized capability output. It will be deserialized by the core.
    /// The `id` MUST match the `id` of the effect that triggered it, else the core will panic.
    // used in docs/internals/bridge.md
    // ANCHOR: handle_response_sig
    pub fn handle_response(&self, id: u32, output: &[u8]) -> Vec<u8>
    // ANCHOR_END: handle_response_sig
    where
        A::Event: for<'a> Deserialize<'a>,
    {
        let options = Self::bincode_options();

        let mut deser = bincode::Deserializer::from_slice(output, options);

        let mut return_buffer = vec![];
        let mut ser = bincode::Serializer::new(&mut return_buffer, options);

        self.inner.handle_response(id, &mut deser, &mut ser);

        return_buffer
    }

    /// Get the current state of the app's view model (serialized).
    pub fn view(&self) -> Vec<u8> {
        let options = Self::bincode_options();

        let mut return_buffer = vec![];

        self.inner
            .view(&mut bincode::Serializer::new(&mut return_buffer, options));

        return_buffer
    }

    fn bincode_options() -> impl bincode::Options + Copy {
        DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes()
    }
}

/// A bridge with a user supplied serializer
///
/// This is exactly the same as [`Bridge`], except instead of using the default
/// bincode serialization, you can provide your own [`Serializer`](bincode::Serializer).
///
/// **Warning**: the support for custom serialization is **experimental** and
/// does not have a corresponding type generation support - you will need
/// to write deserialization code on the shell side yourself, or generate
/// it using separate tooling.
// used in docs/internals/bridge.md
// ANCHOR: bridge_with_serializer
pub struct BridgeWithSerializer<Eff, A>
where
    Eff: Effect,
    A: App,
{
    core: Core<Eff, A>,
    registry: ResolveRegistry,
}
// ANCHOR_END: bridge_with_serializer

impl<Eff, A> BridgeWithSerializer<Eff, A>
where
    Eff: Effect,
    A: App,
{
    pub fn new(core: Core<Eff, A>) -> Self {
        Self {
            core,
            registry: Default::default(),
        }
    }

    /// Receive an event from the shell.
    ///
    /// The `event` is serialized and will be deserialized by the core before it's passed
    /// to your app.
    pub fn process_event<'de, D, S>(&self, event: D, requests_out: S)
    where
        for<'a> A::Event: Deserialize<'a>,
        D: ::serde::de::Deserializer<'de> + 'de,
        S: ::serde::ser::Serializer,
    {
        let mut erased_de = <dyn erased_serde::Deserializer>::erase(event);
        self.process(
            None,
            &mut erased_de,
            &mut <dyn erased_serde::Serializer>::erase(requests_out),
        );
    }

    /// Receive a response to a capability request from the shell.
    ///
    /// The `output` is serialized capability output. It will be deserialized by the core.
    /// The `id` MUST match the `id` of the effect that triggered it, else the core will panic.
    pub fn handle_response<'de, D, S>(&self, id: u32, response: D, requests_out: S)
    where
        for<'a> A::Event: Deserialize<'a>,
        D: ::serde::de::Deserializer<'de>,
        S: ::serde::ser::Serializer,
    {
        let mut erased_response = <dyn erased_serde::Deserializer>::erase(response);
        self.process(
            Some(EffectId(id)),
            &mut erased_response,
            &mut <dyn erased_serde::Serializer>::erase(requests_out),
        );
    }

    fn process(
        &self,
        id: Option<EffectId>,
        data: &mut dyn erased_serde::Deserializer,
        requests_out: &mut dyn erased_serde::Serializer,
    ) where
        A::Event: for<'a> Deserialize<'a>,
    {
        let effects = match id {
            None => {
                let shell_event =
                    erased_serde::deserialize(data).expect("Message deserialization failed.");

                self.core.process_event(shell_event)
            }
            Some(id) => {
                self.registry.resume(id, data).expect(
                    "Response could not be handled. The request did not expect a response.",
                );

                self.core.process()
            }
        };

        let requests: Vec<_> = effects
            .into_iter()
            .map(|eff| self.registry.register(eff))
            .collect();

        requests
            .erased_serialize(requests_out)
            .expect("Request serialization failed.")
    }

    /// Get the current state of the app's view model (serialized).
    pub fn view<S>(&self, ser: S)
    where
        S: ::serde::ser::Serializer,
    {
        self.core
            .view()
            .erased_serialize(&mut <dyn erased_serde::Serializer>::erase(ser))
            .expect("View should serialize")
    }
}
