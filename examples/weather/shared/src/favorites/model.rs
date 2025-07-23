use serde::{Deserialize, Serialize};

use crate::{location::Location, weather::model::CurrentResponse, GeocodingResponse};

pub const FAVORITES_KEY: &str = "favorites";

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
pub struct Favorite {
    pub geo: GeocodingResponse,
    pub current: Option<CurrentResponse>,
}

impl From<GeocodingResponse> for Favorite {
    fn from(geo: GeocodingResponse) -> Self {
        Favorite { geo, current: None }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum FavoritesState {
    Idle,
    ConfirmDelete(Location),
}
