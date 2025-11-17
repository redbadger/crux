pub mod capability;
pub mod client;
pub mod model;

use facet::Facet;
use model::GeocodingResponse;
use serde::{Deserialize, Serialize};

#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
}

impl From<&GeocodingResponse> for Location {
    fn from(value: &GeocodingResponse) -> Self {
        Location {
            lat: value.lat,
            lon: value.lon,
        }
    }
}
