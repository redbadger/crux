//! The persistent favourites collection.
//!
//! A favourite is a geocoded location ([`GeocodingResponse`]) wrapped in a
//! newtype. [`Favorites`] is the ordered collection the app stores under
//! [`FAVORITES_KEY`] in the KV store, serialised as JSON.

use serde::{Deserialize, Serialize};

use crate::effects::location::Location;

use crate::effects::http::location::GeocodingResponse;

/// The key under which the favourites list is stored in the KV store.
pub const FAVORITES_KEY: &str = "favorites";

/// A saved location — a geocoded city the user wants to see weather for.
///
/// `serde(transparent)` means the on-disk representation is just a
/// [`GeocodingResponse`]; the newtype exists for type safety in the model.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(transparent)]
pub struct Favorite(pub GeocodingResponse);

impl From<GeocodingResponse> for Favorite {
    fn from(geo: GeocodingResponse) -> Self {
        Favorite(geo)
    }
}

impl Favorite {
    /// The display name of the favourite (e.g. "Phoenix").
    #[must_use]
    pub fn name(&self) -> &str {
        &self.0.name
    }

    pub(crate) fn location(&self) -> Location {
        self.0.location()
    }
}

/// An ordered collection of [`Favorite`]s, keyed by [`Location`] for
/// add/remove/lookup.
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Favorites(Vec<Favorite>);

impl Favorites {
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Favorite> {
        self.0.iter()
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    #[cfg(test)]
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

    pub(crate) fn exists(&self, location: &Location) -> bool {
        self.0.iter().any(|fav| &fav.location() == location)
    }
}
