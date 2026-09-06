//! Command builders for the [location capability](super).
//!
//! Each builder issues one operation, and the operation's single output is
//! already the type the caller wants — there is nothing to narrow. They're
//! generic over `Effect` and `Event` so they can be reused from any Crux
//! app whose `Effect` type can wrap that request.

use std::future::Future;

use crux_core::{Command, Request, command::RequestBuilder};

use super::{GetLocation, IsLocationEnabled, Location};

/// Asks the shell whether location services are currently enabled.
#[must_use]
pub fn is_location_enabled<Effect, Event>()
-> RequestBuilder<Effect, Event, impl Future<Output = bool>>
where
    Effect: Send + From<Request<IsLocationEnabled>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(IsLocationEnabled)
}

/// Asks the shell for the device's current coordinates.
#[must_use]
pub fn get_location<Effect, Event>()
-> RequestBuilder<Effect, Event, impl Future<Output = Option<Location>>>
where
    Effect: Send + From<Request<GetLocation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(GetLocation)
}
