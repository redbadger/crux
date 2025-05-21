use crux_core::{render::render, Command};
use serde::{Deserialize, Serialize};

use crate::{Effect, Event, GeocodingResponse, Workflow};

use super::favorites::FavoritesState;

#[derive(Serialize, Deserialize, Clone)]
pub enum AddFavoriteEvent {
    Submit(GeocodingResponse),
    Cancel,
}

pub fn update(event: AddFavoriteEvent, model: &mut crate::Model) -> Command<Effect, Event> {
    match event {
        AddFavoriteEvent::Submit(geo) => {
            model.favorites.push(geo.into());
            model.page = Workflow::Favorites(FavoritesState::Idle);
            render()
        }
        AddFavoriteEvent::Cancel => {
            model.page = Workflow::Favorites(FavoritesState::Idle);
            render()
        }
    }
}
