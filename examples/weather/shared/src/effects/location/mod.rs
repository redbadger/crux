pub mod command;

use crux_core::capability::Operation;
use facet::Facet;
use serde::{Deserialize, Serialize};

#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Facet, Clone, Serialize, Deserialize, Debug, PartialEq)]
#[repr(C)]
pub enum LocationOperation {
    IsLocationEnabled,
    GetLocation,
}

#[derive(Facet, Clone, Serialize, Deserialize, Debug, PartialEq)]
#[repr(C)]
pub enum LocationResult {
    Enabled(bool),
    Location(Option<Location>),
}

impl Operation for LocationOperation {
    type Output = LocationResult;
}
