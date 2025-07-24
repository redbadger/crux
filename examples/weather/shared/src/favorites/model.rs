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

impl Favorite {
    pub(crate) fn location(&self) -> Location {
        self.geo.location()
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Favorites(Vec<Favorite>);

impl Favorites {
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Favorite> {
        self.0.iter()
    }

    #[cfg(test)]
    pub(crate) fn get(&self, location: &Location) -> Option<&Favorite> {
        self.0.iter().find(|fav| &fav.geo.location() == location)
    }

    pub(crate) fn update(&mut self, location: &Location, mutation: impl FnOnce(&mut Favorite)) {
        if let Some(fav) = self.0.iter_mut().find(|f| &f.geo.location() == location) {
            mutation(fav);
        }
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn insert(&mut self, value: Favorite) {
        self.0.push(value);
    }

    pub(crate) fn remove(&mut self, location: &Location) -> Option<Favorite> {
        self.0
            .iter()
            .position(|f| &f.location() == location)
            .map(|idx| self.0.remove(idx))
    }

    pub(crate) fn from_vec(vec: Vec<Favorite>) -> Self {
        Self(vec)
    }

    pub(crate) fn as_slice(&self) -> &[Favorite] {
        &self.0
    }
}
