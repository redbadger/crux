use facet::Facet;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Facet, Debug, Serialize, Deserialize, PartialOrd, PartialEq, Default, Copy, Clone)]
pub struct Coord {
    pub lat: f64,
    pub lon: f64,
}

impl fmt::Display for Coord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lat: {}, lon: {}", self.lat, self.lon)
    }
}
