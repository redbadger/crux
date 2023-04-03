mod registry;
pub mod step_serde;

use serde::{Deserialize, Serialize};

use crate::{App, Core, Effect};
use registry::ResolveRegistry;

/// Request for a side-effect passed from the Core to the Shell. The `uuid` links
/// the `Request` with the corresponding call to [`Core::response`] to pass the data back
/// to the [`App::update`] function (wrapped in the event provided to the capability originating the effect).
#[derive(Debug, Serialize, Deserialize)]
pub struct Request<Eff>
where
    Eff: Effect,
{
    pub uuid: Vec<u8>,
    pub effect: Eff::Ffi,
}

pub struct Bridge<'de, Eff, A>
where
    Eff: Effect,
    A: App,
{
    core: Core<Eff, A>,
    registry: ResolveRegistry<'de>,
}

impl<'de, Eff, A> Bridge<'de, Eff, A>
where
    Eff: Effect + Send + 'static,
    A: App,
{
    /// Receive an event from the shell.
    ///
    /// The `event` is serialized and will be deserialized by the core before it's passed
    /// to your app.
    pub fn process_event(&self, event: &'de [u8]) -> Vec<u8>
    where
        <A as App>::Event: Deserialize<'de>,
    {
        self.process(None, event)
    }

    /// Receive a response to a capability request from the shell.
    ///
    /// The `output` is serialized capability output. It will be deserialized by the core.
    /// The `uuid` MUST match the `uuid` of the effect that triggered it, else the core will panic.
    pub fn handle_response(&self, uuid: &[u8], output: &'de [u8]) -> Vec<u8>
    where
        <A as App>::Event: Deserialize<'de>,
    {
        self.process(Some(uuid), output)
    }

    fn process(&self, uuid: Option<&[u8]>, data: &'de [u8]) -> Vec<u8>
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
}
