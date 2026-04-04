pub mod favorites;
pub mod local;

use crux_core::render::render;
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::model::ApiKey;
use crate::model::outcome::{Outcome, Started, Status};

use self::favorites::{FavoriteWeatherEvent, FavoriteWeatherTransition};
use self::local::{LocalWeatherEvent, LocalWeatherTransition};
use super::favorites::model::Favorites;

pub use self::favorites::{FavoriteWeather, FavoriteWeatherState};
pub use self::local::LocalWeather;

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum HomeEvent {
    GoToFavorites,

    #[serde(skip)]
    #[facet(skip)]
    Local(#[facet(opaque)] LocalWeatherEvent),

    #[serde(skip)]
    #[facet(skip)]
    FavoritesWeather(#[facet(opaque)] FavoriteWeatherEvent),
}

#[derive(Debug)]
pub(crate) enum HomeTransition {
    GoToFavorites(Favorites),
    ApiKeyRejected,
}

#[derive(Default, Debug)]
pub struct HomeScreen {
    pub current_weather: LocalWeather,
    pub favorites_weather: Vec<FavoriteWeather>,
}

impl HomeScreen {
    pub(crate) fn start(
        favorites: Favorites,
        api_key: &ApiKey,
    ) -> Started<Self, HomeEvent> {
        tracing::debug!("starting home screen");

        let (current_weather, local_cmd) = LocalWeather::start()
            .map_event(HomeEvent::Local)
            .into_parts();

        let (favorites_weather, fav_cmd) = self::favorites::start(favorites, api_key)
            .map_event(HomeEvent::FavoritesWeather)
            .into_parts();

        let screen = Self {
            current_weather,
            favorites_weather,
        };

        Started::new(screen, local_cmd.and(fav_cmd))
    }

    pub(crate) fn update(
        self,
        event: HomeEvent,
        api_key: &ApiKey,
    ) -> Outcome<Self, HomeTransition, HomeEvent> {
        match event {
            HomeEvent::GoToFavorites => {
                let favorites = Favorites::from_vec(
                    self.favorites_weather
                        .iter()
                        .map(|fw| fw.favorite.clone())
                        .collect(),
                );
                Outcome::complete(HomeTransition::GoToFavorites(favorites), render())
            }

            HomeEvent::Local(local_event) => {
                let Self {
                    current_weather,
                    favorites_weather,
                } = self;

                let (status, cmd) = current_weather
                    .update(local_event, api_key)
                    .map_event(HomeEvent::Local)
                    .into_parts();

                match status {
                    Status::Continue(current_weather) => Outcome::continuing(
                        Self {
                            current_weather,
                            favorites_weather,
                        },
                        cmd,
                    ),
                    Status::Complete(LocalWeatherTransition::Unauthorized) => {
                        Outcome::complete(HomeTransition::ApiKeyRejected, cmd)
                    }
                }
            }

            HomeEvent::FavoritesWeather(fav_event) => {
                let Self {
                    current_weather,
                    favorites_weather,
                } = self;

                let (status, cmd) = self::favorites::update(favorites_weather, fav_event)
                    .map_event(HomeEvent::FavoritesWeather)
                    .into_parts();

                match status {
                    Status::Continue(favorites_weather) => Outcome::continuing(
                        Self {
                            current_weather,
                            favorites_weather,
                        },
                        cmd,
                    ),
                    Status::Complete(FavoriteWeatherTransition::Unauthorized) => {
                        Outcome::complete(HomeTransition::ApiKeyRejected, cmd)
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::ApiKey;
    use crate::model::active::favorites::model::{Favorite, Favorites};
    use crate::effects::http::location::GeocodingResponse;

    use super::*;

    const TEST_API_KEY: &str = "test_api_key";

    fn test_api_key() -> ApiKey {
        TEST_API_KEY.to_string().into()
    }

    fn test_favorite() -> Favorite {
        Favorite(GeocodingResponse {
            name: "Phoenix".to_string(),
            local_names: None,
            lat: 33.456_789,
            lon: -112.037_222,
            country: "US".to_string(),
            state: None,
        })
    }

    #[test]
    fn start_fetches_favorites_weather() {
        let mut favorites = Favorites::default();
        let test_fav = test_favorite();
        favorites.insert(test_fav.clone());

        let api_key = test_api_key();
        let (screen, mut cmd) = HomeScreen::start(favorites, &api_key).into_parts();

        assert_eq!(screen.favorites_weather.len(), 1);
        assert_eq!(
            screen.favorites_weather[0].weather,
            FavoriteWeatherState::Fetching
        );

        // Should have two effects: location check + favorite weather fetch
        let effects: Vec<_> = cmd.effects().collect();
        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn fetch_favorites_triggers_fetch_for_all_favorites() {
        let mut favorites = Favorites::default();
        favorites.insert(test_favorite());
        favorites.insert(Favorite(GeocodingResponse {
            name: "New York".to_string(),
            local_names: None,
            lat: 40.7128,
            lon: -74.0060,
            country: "US".to_string(),
            state: None,
        }));

        let api_key = test_api_key();
        let (screen, mut cmd) = HomeScreen::start(favorites, &api_key).into_parts();

        assert_eq!(screen.favorites_weather.len(), 2);

        // location check + 2 favorite weather fetches
        let effects: Vec<_> = cmd.effects().collect();
        assert_eq!(effects.len(), 3);
    }
}
