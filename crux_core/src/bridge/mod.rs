mod registry;
mod request_serde;

use serde::{Deserialize, Serialize};

use crate::Effect;
use crate::{App, Core};
use registry::ResolveRegistry;
// ResolveByte is public to be accessible from crux_macros
#[doc(hidden)]
pub use request_serde::ResolveBytes;

/// Request for a side-effect passed from the Core to the Shell. The `uuid` links
/// the `Request` with the corresponding call to [`Core::resolve`] to pass the data back
/// to the [`App::update`] function (wrapped in the event provided to the capability originating the effect).
#[derive(Debug, Serialize, Deserialize)]
pub struct Request<Eff>
where
    Eff: Serialize,
{
    pub uuid: Vec<u8>,
    pub effect: Eff,
}

/// Bridge is a core wrapper presenting the same interface as the [`Core`] but in a
/// serialized form
pub struct Bridge<Eff, A>
where
    Eff: Effect,
    A: App,
{
    core: Core<Eff, A>,
    registry: ResolveRegistry,
}

impl<Eff, A> Bridge<Eff, A>
where
    Eff: Effect + Send + 'static,
    A: App,
{
    /// Create a new Bridge using the provided `core`.
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
    pub fn process_event<'de>(&self, event: &'de [u8]) -> Vec<u8>
    where
        <A as App>::Event: Deserialize<'de>,
    {
        self.process(None, event)
    }

    /// Receive a response to a capability request from the shell.
    ///
    /// The `output` is serialized capability output. It will be deserialized by the core.
    /// The `uuid` MUST match the `uuid` of the effect that triggered it, else the core will panic.
    pub fn handle_response<'de>(&self, uuid: &[u8], output: &'de [u8]) -> Vec<u8>
    where
        <A as App>::Event: Deserialize<'de>,
    {
        self.process(Some(uuid), output)
    }

    fn process<'de>(&self, uuid: Option<&[u8]>, data: &'de [u8]) -> Vec<u8>
    where
        <A as App>::Event: Deserialize<'de>,
    {
        let effects = match uuid {
            None => {
                let shell_event = bcs::from_bytes(data).expect("Message deserialization failed.");

                self.core.process_event(shell_event)
            }
            Some(uuid) => {
                self.registry.resume(uuid, data).expect(
                    "Response could not be handled. The request did not expect a response.",
                );

                self.core.process()
            }
        };

        let requests: Vec<_> = effects
            .into_iter()
            .map(|eff| self.registry.register(eff))
            .collect();

        bcs::to_bytes(&requests).expect("Request serialization failed.")
    }

    /// Get the current state of the app's view model (serialized).
    pub fn view(&self) -> Vec<u8> {
        bcs::to_bytes(&self.core.view()).expect("View should serialize")
    }
}
