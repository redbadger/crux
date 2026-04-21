//! Command builders for the [location capability](super).
//!
//! Each builder issues one [`LocationOperation`] and narrows the shell's
//! [`LocationResult`] to the specific type the caller cares about. They're
//! generic over `Effect` and `Event` so they can be reused from any Crux
//! app whose `Effect` type can wrap a location request.

use std::future::Future;

use crux_core::{Command, Request, command::RequestBuilder};

use super::{Location, LocationOperation, LocationResult};

/// Asks the shell whether location services are currently enabled.
#[must_use]
pub fn is_location_enabled<Effect, Event>()
-> RequestBuilder<Effect, Event, impl Future<Output = bool>>
where
    Effect: Send + From<Request<LocationOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(LocationOperation::IsLocationEnabled).map(|result| match result {
        LocationResult::Enabled(val) => val,
        LocationResult::Location(_) => false,
    })
}

/// Asks the shell for the device's current coordinates.
#[must_use]
pub fn get_location<Effect, Event>()
-> RequestBuilder<Effect, Event, impl Future<Output = Option<Location>>>
where
    Effect: Send + From<Request<LocationOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(LocationOperation::GetLocation).map(|result| match result {
        LocationResult::Location(loc) => loc,
        LocationResult::Enabled(_) => None,
    })
}
