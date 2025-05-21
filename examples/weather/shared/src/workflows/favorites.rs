use crux_core::{render::render, Command};
use serde::{Deserialize, Serialize};

use crate::{CurrentResponse, Effect, Event, GeocodingResponse, Workflow};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Favorite {
    #[serde(flatten)]
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
    ConfirmDelete(f64, f64),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum FavoritesEvent {
    AddPressed,
    DeletePressed(Favorite),
    DeleteConfirmed,
    DeleteCancelled,
}

pub fn update(event: FavoritesEvent, model: &mut crate::Model) -> Command<Effect, Event> {
    match event {
        FavoritesEvent::AddPressed => {
            model.page = Workflow::AddFavorite;
            Command::done()
        }

        FavoritesEvent::DeletePressed(favorite) => {
            model.page = Workflow::Favorites(FavoritesState::ConfirmDelete(
                favorite.geo.lat,
                favorite.geo.lon,
            ));
            render()
        }

        FavoritesEvent::DeleteConfirmed => {
            if let Workflow::Favorites(FavoritesState::ConfirmDelete(lat, lng)) = model.page {
                if let Some(index) = model
                    .favorites
                    .iter()
                    .position(|f| f.geo.lat == lat && f.geo.lon == lng)
                {
                    model.favorites.remove(index);
                }
            }
            model.page = Workflow::Favorites(FavoritesState::Idle);
            render()
        }

        FavoritesEvent::DeleteCancelled => {
            model.page = Workflow::Favorites(FavoritesState::Idle);
            render()
        }
    }
}
