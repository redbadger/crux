//! A custom capability for accessing the device's location.
//!
//! Two operations — checking whether location services are enabled and
//! fetching the current coordinates — exchanged with the shell through
//! [`LocationOperation`] and [`LocationResult`]. The developer-facing
//! command builders live in the [`command`] submodule.

pub mod command;

use crux_core::capability::Operation;
use facet::Facet;
use serde::{Deserialize, Serialize};

/// Geographic coordinates as returned by the shell.
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
}

/// Operations the core can ask the shell to perform.
#[derive(Facet, Clone, Serialize, Deserialize, Debug, PartialEq)]
#[repr(C)]
pub enum LocationOperation {
    /// Ask whether location services are currently enabled and authorised.
    IsLocationEnabled,
    /// Ask for the device's current coordinates.
    GetLocation,
}

/// Values the shell can return in response to a [`LocationOperation`].
#[derive(Facet, Clone, Serialize, Deserialize, Debug, PartialEq)]
#[repr(C)]
pub enum LocationResult {
    /// Whether location services are enabled and authorised.
    Enabled(bool),
    /// The current location, or `None` if the shell couldn't determine it.
    Location(Option<Location>),
}

impl Operation for LocationOperation {
    type Output = LocationResult;
}
