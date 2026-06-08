use std::sync::{Arc, Weak};

use serde::{Deserialize, Serialize};

use crate::{
    EffectFFI,
    bridge::{BridgeError, EffectId, FfiFormat, Request as BridgeRequest, ResolveRegistry},
    effects::{EffectRouter, Routes},
};

/// The default route, reproducing the standard [`Bridge`](crate::bridge::Bridge)
/// behaviour on top of the [`EffectRouter`].
///
/// Effects on this lane are serialized to bytes using `Format`, registered under
/// an [`EffectId`](crate::bridge::EffectId), and sent to the shell. The shell
/// later calls [`Serialized::resolve`] with the id and the serialized response.
/// Events and the view model are likewise exchanged as bytes via
/// [`Serialized::update`] and [`Serialized::view`].
///
/// This is the primary onboarding path and typically acts as the fall-through
/// arm of the routing closure, handling every effect that isn't claimed by a
/// more specialised lane.
///
/// `Serialized` keeps a [`Weak`] reference to its [`EffectRouter`] so that
/// resolving a request and processing an event can advance the runtime and
/// route follow-up effects.
pub struct Serialized<App, RouteSet, Format>
where
    App: crate::App,
    RouteSet: Routes<App>,
    Format: FfiFormat,
{
    router: Weak<EffectRouter<App, RouteSet>>,
    registry: ResolveRegistry<Format>,
}

impl<App, RouteSet, Format> Serialized<App, RouteSet, Format>
where
    App: crate::App,
    RouteSet: Routes<App> + Send + Sync + 'static,
    Format: FfiFormat,
{
    /// Create a serialized route attached to `router`.
    ///
    /// Called from your [`Routes::new`] implementation with the [`Weak`] router
    /// handle the trait provides.
    #[must_use]
    pub fn new(router: Weak<EffectRouter<App, RouteSet>>) -> Self {
        Self {
            router,
            registry: ResolveRegistry::default(),
        }
    }

    /// Process a serialized shell event and route any emitted effects.
    ///
    /// # Errors
    ///
    /// Returns an error if the event bytes could not be deserialized.
    ///
    /// # Panics
    ///
    /// Panics if the router has been dropped.
    pub fn update<'a>(&self, event: &'a [u8]) -> Result<(), BridgeError<Format>>
    where
        App::Event: Deserialize<'a>,
    {
        let event = Format::deserialize(event).map_err(BridgeError::DeserializeEvent)?;

        self.router().update(event);

        Ok(())
    }

    /// Park a serialized effect and encode it as request bytes for the shell.
    ///
    /// # Errors
    ///
    /// Returns an error if the request could not be serialized.
    ///
    /// # Panics
    ///
    /// Panics if the internal registry lock has been poisoned.
    pub fn serialize<Eff>(&self, effect: Eff) -> Result<Vec<u8>, BridgeError<Format>>
    where
        Eff: EffectFFI,
    {
        let request = self.registry.register(effect);
        Self::encode_requests(&[request])
    }

    /// Resume a serialized request and route any follow-up effects.
    ///
    /// # Errors
    ///
    /// Returns an error if the request could not be resumed.
    ///
    /// # Panics
    ///
    /// Panics if the router has been dropped or the internal registry lock has
    /// been poisoned.
    pub fn resolve(&self, id: EffectId, response: &[u8]) -> Result<(), BridgeError<Format>> {
        self.registry.resume(id, response)?;
        self.router().process();

        Ok(())
    }

    /// Serialize the current view model.
    ///
    /// # Errors
    ///
    /// Returns an error if the view model could not be serialized.
    ///
    /// # Panics
    ///
    /// Panics if the router has been dropped.
    pub fn view(&self) -> Result<Vec<u8>, BridgeError<Format>>
    where
        App::ViewModel: Serialize,
    {
        let view = self.router().view();
        let mut bytes = Vec::new();

        Format::serialize(&mut bytes, &view).map_err(BridgeError::SerializeView)?;

        Ok(bytes)
    }

    fn router(&self) -> Arc<EffectRouter<App, RouteSet>> {
        self.router.upgrade().expect("effect router dropped")
    }

    fn encode_requests<Eff>(requests: &[BridgeRequest<Eff>]) -> Result<Vec<u8>, BridgeError<Format>>
    where
        Eff: Serialize,
    {
        let mut bytes = Vec::new();

        Format::serialize(&mut bytes, &requests).map_err(BridgeError::SerializeRequests)?;

        Ok(bytes)
    }
}
