pub mod client;

use std::future::Future;

use crux_core::{Command, Request, capability::Operation, command::RequestBuilder};
use facet::Facet;
use serde::{Deserialize, Serialize};

#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
}

// The operations that can be performed related to location.
// Using an enum allows us to easily add more operations in the future and ensures type safety.
#[derive(Facet, Clone, Serialize, Deserialize, Debug, PartialEq)]
#[repr(C)]
pub enum LocationOperation {
    IsLocationEnabled,
    GetLocation,
}

// The possible results from performing a location operation.
// This enum allows us to handle different response types in a type-safe way.
#[derive(Facet, Clone, Serialize, Deserialize, Debug, PartialEq)]
#[repr(C)]
pub enum LocationResult {
    Enabled(bool),
    Location(Option<Location>),
}

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

// Implement the Operation trait so that Crux knows how to handle this effect.
// This ties the operation type to its output/result type.
impl Operation for LocationOperation {
    type Output = LocationResult;
}
