use crux_core::{Command, render::render};
use facet::Facet;
use serde::{Deserialize, Serialize};

pub mod weather;

use crate::effects::Effect;
use crate::effects::location::Location;
use crate::effects::location::command::get_location;
use crate::model::ApiKey;
use crate::model::outcome::Outcome;

use self::weather::client::{WeatherApi, WeatherError};
use self::weather::model::current_response::CurrentWeatherResponse;
use super::favorites::model::{Favorite, Favorites};

// -- Events --

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum HomeEvent {
    GoToFavorites,

    #[serde(skip)]
    #[facet(skip)]
    LocationEnabled(bool),

    #[serde(skip)]
    #[facet(skip)]
    LocationFetched(Option<Location>),

    #[serde(skip)]
    #[facet(skip)]
    WeatherFetched(#[facet(opaque)] Box<Result<CurrentWeatherResponse, WeatherError>>),

    #[serde(skip)]
    #[facet(skip)]
    FavoriteWeatherFetched(
        #[facet(opaque)] Box<Result<CurrentWeatherResponse, WeatherError>>,
        Location,
    ),
}

// -- Transitions --

#[derive(Debug)]
pub(crate) enum HomeTransition {
    GoToFavorites(Favorites),
    Unauthorized,
}

// -- State --

#[derive(Debug, Clone)]
pub enum LocalWeather {
    CheckingPermission,
    LocationDisabled,
    FetchingLocation,
    FetchingWeather(Location),
    Fetched(Location, CurrentWeatherResponse),
    Failed(Location),
}

impl Default for LocalWeather {
    fn default() -> Self {
        Self::CheckingPermission
    }
}

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

#[derive(Default, Debug)]
pub struct HomeScreen {
    pub current_weather: LocalWeather,
    pub favorites_weather: Vec<FavoriteWeather>,
}

impl HomeScreen {
    pub(crate) fn start(
        favorites: Favorites,
        api_key: &ApiKey,
    ) -> (Self, Command<Effect, HomeEvent>) {
        tracing::debug!("starting home screen, checking location permissions");

        let favorites_weather: Vec<FavoriteWeather> = favorites
            .iter()
            .map(|f| FavoriteWeather {
                favorite: f.clone(),
                weather: FavoriteWeatherState::Fetching,
            })
            .collect();

        let screen = Self {
            current_weather: LocalWeather::CheckingPermission,
            favorites_weather,
        };

        // Flow 1: local weather — check location permission
        let location_cmd = crate::effects::location::command::is_location_enabled()
            .then_send(HomeEvent::LocationEnabled);

        // Flow 2: favorites weather — fetch all in parallel
        let favorites_cmd = screen.fetch_favorites_weather(api_key);

        (screen, location_cmd.and(favorites_cmd))
    }

    pub(crate) fn update(
        mut self,
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

            // -- Flow 1: local weather --

            HomeEvent::LocationEnabled(enabled) => {
                tracing::debug!("location enabled: {enabled}");
                if enabled {
                    self.current_weather = LocalWeather::FetchingLocation;
                    tracing::debug!("fetching current location");
                    let cmd = get_location().then_send(HomeEvent::LocationFetched);
                    Outcome::continuing(self, cmd)
                } else {
                    self.current_weather = LocalWeather::LocationDisabled;
                    Outcome::continuing(self, render())
                }
            }
            HomeEvent::LocationFetched(location) => {
                tracing::debug!("received location: {location:?}");
                match location {
                    Some(loc) => {
                        self.current_weather = LocalWeather::FetchingWeather(loc);
                        let cmd = WeatherApi::fetch(loc, api_key.clone())
                            .then_send(|result| HomeEvent::WeatherFetched(Box::new(result)));
                        Outcome::continuing(self, cmd)
                    }
                    None => {
                        self.current_weather = LocalWeather::LocationDisabled;
                        Outcome::continuing(self, render())
                    }
                }
            }
            HomeEvent::WeatherFetched(result) => {
                let LocalWeather::FetchingWeather(location) = self.current_weather else {
                    return Outcome::continuing(self, Command::done());
                };

                match *result {
                    Ok(weather_data) => {
                        tracing::debug!("received weather data for {}", weather_data.name);
                        self.current_weather =
                            LocalWeather::Fetched(location, weather_data);
                    }
                    Err(WeatherError::Unauthorized) => {
                        return Outcome::complete(HomeTransition::Unauthorized, render());
                    }
                    Err(ref e) => {
                        tracing::debug!("fetching weather failed: {e:?}");
                        self.current_weather = LocalWeather::Failed(location);
                    }
                }

                Outcome::continuing(self, render())
            }

            // -- Flow 2: favorites weather --

            HomeEvent::FavoriteWeatherFetched(result, location) => {
                match *result {
                    Ok(weather) => {
                        tracing::debug!(
                            "received favorite weather for ({}, {})",
                            location.lat,
                            location.lon
                        );
                        if let Some(fw) = self
                            .favorites_weather
                            .iter_mut()
                            .find(|fw| fw.favorite.location() == location)
                        {
                            fw.weather = FavoriteWeatherState::Fetched(weather);
                        }
                    }
                    Err(WeatherError::Unauthorized) => {
                        return Outcome::complete(HomeTransition::Unauthorized, render());
                    }
                    Err(_) => {
                        if let Some(fw) = self
                            .favorites_weather
                            .iter_mut()
                            .find(|fw| fw.favorite.location() == location)
                        {
                            fw.weather = FavoriteWeatherState::Failed;
                        }
                    }
                }
                Outcome::continuing(self, render())
            }
        }
    }

    fn fetch_favorites_weather(&self, api_key: &ApiKey) -> Command<Effect, HomeEvent> {
        if self.favorites_weather.is_empty() {
            return Command::done();
        }

        tracing::debug!(
            "fetching weather for {} favorites",
            self.favorites_weather.len()
        );
        self.favorites_weather
            .iter()
            .map(|fw| {
                let location = fw.favorite.location();
                let api_key = api_key.clone();

                WeatherApi::fetch(location, api_key).then_send(move |result| {
                    HomeEvent::FavoriteWeatherFetched(Box::new(result), location)
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crux_http::protocol::{HttpResponse, HttpResult};

    use crate::{
        effects::location::{Location, LocationOperation, LocationResult},
        model::ApiKey,
    };

    use crate::model::active::favorites::model::Favorite;
    use crate::model::active::location::GeocodingResponse;

    use super::weather::client::WeatherApi;
    use super::weather::model::{
        current_response::{
            CurrentWeatherResponse, CurrentWeatherResponseBuilder, Main, Sys,
        },
        response_elements::{Clouds, Coord, WeatherData, Wind},
    };
    use super::*;

    const TEST_API_KEY: &str = "test_api_key";

    fn test_api_key() -> ApiKey {
        TEST_API_KEY.to_string().into()
    }

    fn test_screen() -> HomeScreen {
        HomeScreen::default()
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

    fn test_response_json() -> String {
        serde_json::to_string(&test_response()).unwrap()
    }

    #[test]
    fn location_enabled_fetches_weather() {
        let screen = test_screen();
        let api_key = test_api_key();

        let (screen, mut cmd) = screen
            .update(HomeEvent::LocationEnabled(true), &api_key)
            .expect_continue()
            .into_parts();

        assert!(matches!(
            screen.current_weather,
            LocalWeather::FetchingLocation
        ));

        let mut location = cmd.expect_one_effect().expect_location();
        assert_eq!(location.operation, LocationOperation::GetLocation);

        let test_location = Location {
            lat: 33.456_789,
            lon: -112.037_222,
        };
        location
            .resolve(LocationResult::Location(Some(test_location)))
            .expect("to resolve");

        let event = cmd.expect_one_event();
        let (screen, mut cmd) = screen
            .update(event, &api_key)
            .expect_continue()
            .into_parts();

        assert!(matches!(
            screen.current_weather,
            LocalWeather::FetchingWeather(_)
        ));

        let mut request = cmd.expect_one_effect().expect_http();
        assert_eq!(
            &request.operation,
            &WeatherApi::build(test_location, &test_api_key())
        );

        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(test_response_json().as_bytes())
                    .build(),
            ))
            .unwrap();

        let actual = cmd.expect_one_event();
        assert!(matches!(actual, HomeEvent::WeatherFetched(_)));

        let (screen, _cmd) = screen
            .update(actual, &api_key)
            .expect_continue()
            .into_parts();

        assert!(matches!(
            screen.current_weather,
            LocalWeather::Fetched(_, _)
        ));
        if let LocalWeather::Fetched(_, ref data) = screen.current_weather {
            assert_eq!(data, &test_response());
        }
    }

    #[test]
    fn location_disabled() {
        let screen = test_screen();
        let api_key = test_api_key();

        let (screen, _cmd) = screen
            .update(HomeEvent::LocationEnabled(false), &api_key)
            .expect_continue()
            .into_parts();

        assert!(matches!(
            screen.current_weather,
            LocalWeather::LocationDisabled
        ));
    }

    #[test]
    fn location_fetched_triggers_weather_fetch() {
        let screen = test_screen();
        let api_key = test_api_key();

        let lat_lon = Location {
            lat: 33.456_789,
            lon: 112.037_222,
        };

        let (screen, mut cmd) = screen
            .update(HomeEvent::LocationFetched(Some(lat_lon)), &api_key)
            .expect_continue()
            .into_parts();

        assert!(matches!(
            screen.current_weather,
            LocalWeather::FetchingWeather(_)
        ));

        let mut request = cmd.effects().next().unwrap().expect_http();
        assert_eq!(
            &request.operation,
            &WeatherApi::build(lat_lon, &test_api_key())
        );

        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(test_response_json().as_bytes())
                    .build(),
            ))
            .unwrap();

        let actual = cmd.events().next().unwrap();
        assert!(matches!(actual, HomeEvent::WeatherFetched(_)));

        let (screen, _cmd) = screen
            .update(actual, &api_key)
            .expect_continue()
            .into_parts();

        if let LocalWeather::Fetched(_, ref data) = screen.current_weather {
            assert_eq!(data, &test_response());
            insta::assert_yaml_snapshot!(data);
        } else {
            panic!("Expected Fetched state");
        }
    }

    #[test]
    fn start_fetches_favorites_weather() {
        let mut favorites = Favorites::default();
        let test_fav = test_favorite();
        favorites.insert(test_fav.clone());

        let api_key = test_api_key();
        let (screen, mut cmd) = HomeScreen::start(favorites, &api_key);

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
    fn favorite_weather_fetched_updates_state() {
        let mut favorites = Favorites::default();
        let test_fav = test_favorite();
        favorites.insert(test_fav.clone());

        let api_key = test_api_key();
        let (screen, _cmd) = HomeScreen::start(favorites, &api_key);

        let location = test_fav.location();
        let (screen, _cmd) = screen
            .update(
                HomeEvent::FavoriteWeatherFetched(
                    Box::new(Ok(test_response())),
                    location,
                ),
                &api_key,
            )
            .expect_continue()
            .into_parts();

        assert_eq!(
            screen.favorites_weather[0].weather,
            FavoriteWeatherState::Fetched(test_response())
        );
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
        let (screen, mut cmd) = HomeScreen::start(favorites, &api_key);

        assert_eq!(screen.favorites_weather.len(), 2);

        // location check + 2 favorite weather fetches
        let effects: Vec<_> = cmd.effects().collect();
        assert_eq!(effects.len(), 3);
    }
}
