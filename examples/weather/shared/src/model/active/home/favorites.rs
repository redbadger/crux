//! Per-favourite weather fetches shown on the home screen.
//!
//! Given a list of favourites, fires one weather request per entry in
//! parallel and tracks each one independently. Unlike [`LocalWeather`], this
//! workflow has no permission or location-fetch steps — just per-favourite
//! HTTP requests that resolve into [`FavoriteWeatherState`].
//!
//! Exposed as free functions rather than methods because the state is a plain
//! `Vec<FavoriteWeather>` shared with the parent screen.
//!
//! [`LocalWeather`]: super::local::LocalWeather

use crux_core::{Command, render::render};

use crate::effects::Effect;
use crate::effects::location::Location;
use crate::model::ApiKey;
use crate::model::active::favorites::model::{Favorite, Favorites};
use crate::model::outcome::{Outcome, Started};

use crate::effects::http::weather::model::current_response::CurrentWeatherResponse;
use crate::effects::http::weather::{self as weather_api, WeatherError};

/// Events emitted as each per-favourite weather request resolves.
#[derive(Clone, Debug, PartialEq)]
pub enum FavoriteWeatherEvent {
    /// Weather for the favourite at `location` resolved — either a response
    /// or an error.
    WeatherFetched(Box<Result<CurrentWeatherResponse, WeatherError>>, Location),
}

/// The exit from the favourites-weather workflow.
#[derive(Debug)]
pub(crate) enum FavoriteWeatherTransition {
    /// One of the weather requests returned 401; the parent should route
    /// back through onboarding, carrying the favourites along.
    Unauthorized(Favorites),
}

/// Per-favourite fetch state.
#[derive(Debug, Clone, PartialEq)]
pub enum FavoriteWeatherState {
    /// Weather request is in flight.
    Fetching,
    /// Weather received for this favourite.
    Fetched(Box<CurrentWeatherResponse>),
    /// Weather fetch failed for reasons other than unauthorized.
    Failed,
}

/// A favourite paired with the state of its weather fetch.
#[derive(Debug, Clone)]
pub struct FavoriteWeather {
    pub favorite: Favorite,
    pub weather: FavoriteWeatherState,
}

impl From<Vec<FavoriteWeather>> for Favorites {
    fn from(weather: Vec<FavoriteWeather>) -> Self {
        Self::from_vec(weather.into_iter().map(|fw| fw.favorite).collect())
    }
}

/// Initialises a per-favourite list in `Fetching` state and kicks off every
/// weather request in parallel.
pub(crate) fn start(
    favorites: &Favorites,
    api_key: &ApiKey,
) -> Started<Vec<FavoriteWeather>, FavoriteWeatherEvent> {
    let items: Vec<FavoriteWeather> = favorites
        .iter()
        .map(|f| FavoriteWeather {
            favorite: f.clone(),
            weather: FavoriteWeatherState::Fetching,
        })
        .collect();

    let cmd = fetch_all(&items, api_key);
    Started::new(items, cmd)
}

/// Applies a resolved weather response to the matching favourite. A response
/// for an unknown location (e.g. a favourite that was removed mid-flight) is
/// logged and dropped. A 401 anywhere in the batch completes the workflow
/// with [`FavoriteWeatherTransition::Unauthorized`].
pub(crate) fn update(
    mut items: Vec<FavoriteWeather>,
    event: FavoriteWeatherEvent,
) -> Outcome<Vec<FavoriteWeather>, FavoriteWeatherTransition, FavoriteWeatherEvent> {
    match event {
        FavoriteWeatherEvent::WeatherFetched(result, location) => {
            match *result {
                Ok(weather) => {
                    tracing::debug!(
                        "received favorite weather for ({}, {})",
                        location.lat,
                        location.lon
                    );
                    if let Some(fw) = items
                        .iter_mut()
                        .find(|fw| fw.favorite.location() == location)
                    {
                        fw.weather = FavoriteWeatherState::Fetched(Box::new(weather));
                    } else {
                        tracing::warn!(
                            "ignoring weather for unknown favorite ({}, {})",
                            location.lat,
                            location.lon
                        );
                    }
                }
                Err(WeatherError::Unauthorized) => {
                    tracing::warn!("weather API returned unauthorized");
                    return Outcome::complete(
                        FavoriteWeatherTransition::Unauthorized(items.into()),
                        render(),
                    );
                }
                Err(ref e) => {
                    tracing::warn!("fetching favorite weather failed: {e:?}");
                    if let Some(fw) = items
                        .iter_mut()
                        .find(|fw| fw.favorite.location() == location)
                    {
                        fw.weather = FavoriteWeatherState::Failed;
                    } else {
                        tracing::warn!(
                            "ignoring error for unknown favorite ({}, {})",
                            location.lat,
                            location.lon
                        );
                    }
                }
            }
            Outcome::continuing(items, render())
        }
    }
}

fn fetch_all(items: &[FavoriteWeather], api_key: &ApiKey) -> Command<Effect, FavoriteWeatherEvent> {
    if items.is_empty() {
        return Command::done();
    }

    tracing::debug!("fetching weather for {} favorites", items.len());
    items
        .iter()
        .map(|fw| {
            let location = fw.favorite.location();
            let api_key = api_key.clone();

            weather_api::fetch(location, api_key).then_send(move |result| {
                FavoriteWeatherEvent::WeatherFetched(Box::new(result), location)
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::effects::http::location::GeocodingResponse;
    use crate::effects::http::weather::model::{
        current_response::{CurrentWeatherResponse, CurrentWeatherResponseBuilder, Main, Sys},
        response_elements::{Clouds, Coord, WeatherData, Wind},
    };
    use crate::model::ApiKey;
    use crate::model::active::favorites::model::{Favorite, Favorites};

    use super::*;

    const TEST_API_KEY: &str = "test_api_key";

    fn api_key() -> ApiKey {
        TEST_API_KEY.to_string().into()
    }

    fn phoenix_favorite() -> Favorite {
        Favorite(GeocodingResponse {
            name: "Phoenix".to_string(),
            local_names: None,
            lat: 33.456_789,
            lon: -112.037_222,
            country: "US".to_string(),
            state: None,
        })
    }

    fn phoenix_weather_response() -> CurrentWeatherResponse {
        let fav = phoenix_favorite();
        CurrentWeatherResponseBuilder::default()
            .main(Main {
                temp: 20.0,
                feels_like: 18.0,
                temp_min: 18.0,
                temp_max: 22.0,
                pressure: 1013,
                humidity: 50,
            })
            .coord(Coord {
                lat: fav.0.lat,
                lon: fav.0.lon,
            })
            .weather(vec![WeatherData {
                id: 800,
                main: "Clear".to_string(),
                description: "clear sky".to_string(),
                icon: "01d".to_string(),
            }])
            .base(String::new())
            .visibility(10000_usize)
            .wind(Wind {
                speed: 4.1,
                deg: 280,
                gust: Some(5.2),
            })
            .clouds(Clouds { all: 0 })
            .dt(1_716_216_000_usize)
            .sys(Sys {
                id: 1,
                country: "US".to_string(),
                type_: 1,
                sunrise: 1_716_216_000,
                sunset: 1_716_216_000,
            })
            .timezone(1)
            .id(1_usize)
            .name("Phoenix".to_string())
            .cod(200_usize)
            .build()
            .expect("Failed to build sample response")
    }

    fn favorites_with_phoenix() -> (Favorites, Favorite) {
        let mut favorites = Favorites::default();
        let fav = phoenix_favorite();
        favorites.insert(fav.clone());
        (favorites, fav)
    }

    #[test]
    fn weather_fetched_updates_state() {
        let (favorites, fav) = favorites_with_phoenix();
        let items = start(&favorites, &api_key()).into_value();

        let location = fav.location();
        let (items, _cmd) = update(
            items,
            FavoriteWeatherEvent::WeatherFetched(
                Box::new(Ok(phoenix_weather_response())),
                location,
            ),
        )
        .expect_continue()
        .into_parts();

        assert_eq!(
            items[0].weather,
            FavoriteWeatherState::Fetched(Box::new(phoenix_weather_response()))
        );
    }

    #[test]
    fn unauthorized_completes_with_transition() {
        let (favorites, fav) = favorites_with_phoenix();
        let items = start(&favorites, &api_key()).into_value();

        let location = fav.location();
        let (transition, _cmd) = update(
            items,
            FavoriteWeatherEvent::WeatherFetched(
                Box::new(Err(WeatherError::Unauthorized)),
                location,
            ),
        )
        .expect_complete()
        .into_parts();

        assert!(matches!(
            transition,
            FavoriteWeatherTransition::Unauthorized(_)
        ));
    }

    #[test]
    fn network_error_marks_failed() {
        let (favorites, fav) = favorites_with_phoenix();
        let items = start(&favorites, &api_key()).into_value();

        let location = fav.location();
        let (items, _cmd) = update(
            items,
            FavoriteWeatherEvent::WeatherFetched(
                Box::new(Err(WeatherError::NetworkError)),
                location,
            ),
        )
        .expect_continue()
        .into_parts();

        assert_eq!(items[0].weather, FavoriteWeatherState::Failed);
    }

    #[test]
    fn empty_favorites_starts_with_no_effects() {
        let favorites = Favorites::default();
        let Started { state, .. } = start(&favorites, &api_key());

        assert!(state.is_empty());
    }
}
