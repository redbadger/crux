// This module defines the effect for accessing location information in a cross-platform way using Crux.
// The structure here is designed to be serializable, portable, and to fit into Crux's command/request architecture.

use crux_core::capability::CapabilityContext;
use crux_core::capability::Operation;
use serde::{Deserialize, Serialize};

// The operations that can be performed related to location.
// Using an enum allows us to easily add more operations in the future and ensures type safety.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum LocationOperation {
    IsLocationEnabled,
    GetLocation,
}

// The response structure for a location request.
// This is serializable so it can be sent across the FFI boundary.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct LocationResponse {
    pub lat: f64,
    pub lon: f64,
}

// The possible results from performing a location operation.
// This enum allows us to handle different response types in a type-safe way.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum LocationResult {
    Enabled(bool),
    Location(Option<LocationResponse>),
}

pub struct Location<Ev> {
    context: CapabilityContext<LocationOperation, Ev>,
}

impl<Ev> Location<Ev>
where
    Ev: 'static,
{
    #[must_use]
    pub fn new(context: CapabilityContext<LocationOperation, Ev>) -> Self {
        Self { context }
    }

    pub fn is_location_enabled<F>(&self, make_event: F)
    where
        F: FnOnce(bool) -> Ev + Send + Sync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            async move {
                // Send the request to the shell and await the result
                let result = context
                    .request_from_shell(LocationOperation::IsLocationEnabled)
                    .await;
                // Match on the result
                let enabled = match result {
                    LocationResult::Enabled(val) => val,
                    LocationResult::Location(_) => false, // fallback for unexpected result
                };
                // Call make_event and update the app
                context.update_app(make_event(enabled));
            }
        });
    }

    pub fn get_location<F>(&self, make_event: F)
    where
        F: FnOnce(Option<LocationResponse>) -> Ev + Send + Sync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            async move {
                let result = context
                    .request_from_shell(LocationOperation::GetLocation)
                    .await;
                let loc = match result {
                    LocationResult::Location(loc) => loc,
                    LocationResult::Enabled(_) => None,
                };
                context.update_app(make_event(loc));
            }
        });
    }
}

// Implement the Operation trait so that Crux knows how to handle this effect.
// This ties the operation type to its output/result type.
impl Operation for LocationOperation {
    type Output = LocationResult;

    // This is only used for type generation (e.g., for FFI bindings).
    #[cfg(feature = "typegen")]
    fn register_types(generator: &mut crux_core::typegen::TypeGen) -> crux_core::typegen::Result {
        generator.register_type::<Self>()?;
        generator.register_type::<LocationResponse>()?;
        generator.register_type::<LocationResult>()?;
        Ok(())
    }
}
