// This module defines the effect for accessing location information in a cross-platform way using Crux.
// The structure here is designed to be serializable, portable, and to fit into Crux's command/request architecture.

use std::future::Future;

use crux_core::{capability::Operation, command::RequestBuilder, Command, Request};
use serde::{Deserialize, Serialize};

use super::Location;

// The operations that can be performed related to location.
// Using an enum allows us to easily add more operations in the future and ensures type safety.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum LocationOperation {
    IsLocationEnabled,
    GetLocation,
}

// The response structure for a location request.
// This is serializable so it can be sent across the FFI boundary.

// The possible results from performing a location operation.
// This enum allows us to handle different response types in a type-safe way.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum LocationResult {
    Enabled(bool),
    Location(Option<Location>),
}

#[must_use]
pub fn is_location_enabled<Effect, Event>(
) -> RequestBuilder<Effect, Event, impl Future<Output = bool>>
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
pub fn get_location<Effect, Event>(
) -> RequestBuilder<Effect, Event, impl Future<Output = Option<Location>>>
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

    // This is only used for type generation (e.g., for FFI bindings).
    #[cfg(feature = "typegen")]
    fn register_types(generator: &mut crux_core::typegen::TypeGen) -> crux_core::typegen::Result {
        generator.register_type::<Self>()?;
        generator.register_type::<Location>()?;
        generator.register_type::<LocationResult>()?;
        Ok(())
    }
}
