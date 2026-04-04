use std::future::Future;

use crux_core::{Command, Request, command::RequestBuilder};

use super::{Location, LocationOperation, LocationResult};

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
