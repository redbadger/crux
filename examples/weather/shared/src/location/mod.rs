pub mod capability;
pub mod client;
pub mod model;

pub use capability::{
    get_location, is_location_enabled, LocationOperation, LocationResponse, LocationResult,
};
use model::GeocodingResponse;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
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
