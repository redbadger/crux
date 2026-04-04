pub mod client;
pub mod events;
pub mod model;

use crux_core::render::render;

use crate::effects::location::Location;
use crate::model::ApiKey;
use crate::model::outcome::Outcome;

use self::events::FavoritesEvent;
use self::model::Favorites;
use super::location::GeocodingResponse;

#[derive(Debug)]
pub struct FavoritesScreen {
    pub favorites: Favorites,
    pub workflow: Option<FavoritesWorkflow>,
}

#[derive(Debug)]
pub enum FavoritesWorkflow {
    AddFavorite {
        search_results: Option<Vec<GeocodingResponse>>,
    },
    ConfirmDelete(Location),
}

#[derive(Debug)]
pub(crate) enum FavoritesTransition {
    GoToHome(Favorites),
}

impl FavoritesScreen {
    pub(crate) fn update(
        mut self,
        event: FavoritesEvent,
        api_key: &ApiKey,
    ) -> Outcome<Self, FavoritesTransition, FavoritesEvent> {
        match event {
            FavoritesEvent::GoToHome => {
                Outcome::complete(FavoritesTransition::GoToHome(self.favorites), render())
            }
            FavoritesEvent::GoToAddFavorite => {
                self.workflow = Some(FavoritesWorkflow::AddFavorite {
                    search_results: None,
                });
                Outcome::continuing(self, render())
            }
            other => {
                let cmd = events::update(other, &mut self, api_key);
                Outcome::continuing(self, cmd)
            }
        }
    }
}
