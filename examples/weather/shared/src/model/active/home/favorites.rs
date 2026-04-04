use crux_core::{Command, render::render};

use crate::effects::Effect;
use crate::effects::location::Location;
use crate::model::ApiKey;
use crate::model::active::favorites::model::{Favorite, Favorites};
use crate::model::outcome::{Outcome, Started};

use crate::effects::http::weather::{self as weather_api, WeatherError};
use crate::effects::http::weather::model::current_response::CurrentWeatherResponse;

// -- Events --

#[derive(Clone, Debug, PartialEq)]
pub enum FavoriteWeatherEvent {
    WeatherFetched(Box<Result<CurrentWeatherResponse, WeatherError>>, Location),
}

// -- Transitions --

#[derive(Debug)]
pub(crate) enum FavoriteWeatherTransition {
    Unauthorized,
}

// -- State --

#[derive(Debug, Clone, PartialEq)]
pub enum FavoriteWeatherState {
    Fetching,
    Fetched(CurrentWeatherResponse),
    Failed,
}

#[derive(Debug, Clone)]
pub struct FavoriteWeather {
    pub favorite: Favorite,
    pub weather: FavoriteWeatherState,
}

// -- Logic --

pub(crate) fn start(
    favorites: Favorites,
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
                        fw.weather = FavoriteWeatherState::Fetched(weather);
                    }
                }
                Err(WeatherError::Unauthorized) => {
                    return Outcome::complete(FavoriteWeatherTransition::Unauthorized, render());
                }
                Err(_) => {
                    if let Some(fw) = items
                        .iter_mut()
                        .find(|fw| fw.favorite.location() == location)
                    {
                        fw.weather = FavoriteWeatherState::Failed;
                    }
                }
            }
            Outcome::continuing(items, render())
        }
    }
}

fn fetch_all(
    items: &[FavoriteWeather],
    api_key: &ApiKey,
) -> Command<Effect, FavoriteWeatherEvent> {
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
    use crate::model::ApiKey;
    use crate::model::active::favorites::model::{Favorite, Favorites};
    use crate::effects::http::location::GeocodingResponse;

    use crate::effects::http::weather::model::{
        current_response::{CurrentWeatherResponse, CurrentWeatherResponseBuilder, Main, Sys},
        response_elements::{Clouds, Coord, WeatherData, Wind},
    };
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

    fn test_response() -> CurrentWeatherResponse {
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
                lat: 33.456_789,
                lon: -112.037_222,
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

    #[test]
    fn weather_fetched_updates_state() {
        let mut favorites = Favorites::default();
        let test_fav = test_favorite();
        favorites.insert(test_fav.clone());

        let api_key = test_api_key();
        let items = start(favorites, &api_key).into_state();

        let location = test_fav.location();
        let (items, _cmd) = update(
            items,
            FavoriteWeatherEvent::WeatherFetched(Box::new(Ok(test_response())), location),
        )
        .expect_continue()
        .into_parts();

        assert_eq!(
            items[0].weather,
            FavoriteWeatherState::Fetched(test_response())
        );
    }
}
