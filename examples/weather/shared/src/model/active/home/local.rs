use crux_core::{Command, render::render};

use crate::effects::http::weather::{self as weather_api, WeatherError};
use crate::effects::http::weather::model::current_response::CurrentWeatherResponse;
use crate::effects::location::Location;
use crate::effects::location::command::get_location;
use crate::model::ApiKey;
use crate::model::outcome::{Outcome, Started};

// -- Events --

#[derive(Clone, Debug, PartialEq)]
pub enum LocalWeatherEvent {
    LocationEnabled(bool),
    LocationFetched(Option<Location>),
    WeatherFetched(Box<Result<CurrentWeatherResponse, WeatherError>>),
}

// -- Transitions --

#[derive(Debug)]
pub(crate) enum LocalWeatherTransition {
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

impl LocalWeather {
    pub(crate) fn start() -> Started<Self, LocalWeatherEvent> {
        tracing::debug!("checking location permissions");
        let cmd = crate::effects::location::command::is_location_enabled()
            .then_send(LocalWeatherEvent::LocationEnabled);
        Started::new(Self::CheckingPermission, cmd)
    }

    pub(crate) fn update(
        self,
        event: LocalWeatherEvent,
        api_key: &ApiKey,
    ) -> Outcome<Self, LocalWeatherTransition, LocalWeatherEvent> {
        match event {
            LocalWeatherEvent::LocationEnabled(enabled) => {
                tracing::debug!("location enabled: {enabled}");
                if enabled {
                    tracing::debug!("fetching current location");
                    let cmd = get_location().then_send(LocalWeatherEvent::LocationFetched);
                    Outcome::continuing(Self::FetchingLocation, cmd)
                } else {
                    Outcome::continuing(Self::LocationDisabled, render())
                }
            }
            LocalWeatherEvent::LocationFetched(location) => {
                tracing::debug!("received location: {location:?}");
                match location {
                    Some(loc) => {
                        let cmd = weather_api::fetch(loc, api_key.clone()).then_send(|result| {
                            LocalWeatherEvent::WeatherFetched(Box::new(result))
                        });
                        Outcome::continuing(Self::FetchingWeather(loc), cmd)
                    }
                    None => Outcome::continuing(Self::LocationDisabled, render()),
                }
            }
            LocalWeatherEvent::WeatherFetched(result) => {
                let Self::FetchingWeather(location) = self else {
                    return Outcome::continuing(self, Command::done());
                };

                match *result {
                    Ok(weather_data) => {
                        tracing::debug!("received weather data for {}", weather_data.name);
                        Outcome::continuing(Self::Fetched(location, weather_data), render())
                    }
                    Err(WeatherError::Unauthorized) => {
                        Outcome::complete(LocalWeatherTransition::Unauthorized, render())
                    }
                    Err(ref e) => {
                        tracing::debug!("fetching weather failed: {e:?}");
                        Outcome::continuing(Self::Failed(location), render())
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crux_http::protocol::{HttpResponse, HttpResult};

    use crate::{
        effects::location::{Location, LocationOperation, LocationResult},
        model::ApiKey,
    };

    use crate::effects::http::weather;
    use crate::effects::http::weather::model::{
        current_response::{CurrentWeatherResponse, CurrentWeatherResponseBuilder, Main, Sys},
        response_elements::{Clouds, Coord, WeatherData, Wind},
    };
    use super::*;

    const TEST_API_KEY: &str = "test_api_key";

    fn test_api_key() -> ApiKey {
        TEST_API_KEY.to_string().into()
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
        let local = LocalWeather::default();
        let api_key = test_api_key();

        let (local, mut cmd) = local
            .update(LocalWeatherEvent::LocationEnabled(true), &api_key)
            .expect_continue()
            .into_parts();

        assert!(matches!(local, LocalWeather::FetchingLocation));

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
        let (local, mut cmd) = local
            .update(event, &api_key)
            .expect_continue()
            .into_parts();

        assert!(matches!(local, LocalWeather::FetchingWeather(_)));

        let mut request = cmd.expect_one_effect().expect_http();
        assert_eq!(
            &request.operation,
            &weather::build_request(test_location, &test_api_key())
        );

        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(test_response_json().as_bytes())
                    .build(),
            ))
            .unwrap();

        let actual = cmd.expect_one_event();
        assert!(matches!(actual, LocalWeatherEvent::WeatherFetched(_)));

        let (local, _cmd) = local
            .update(actual, &api_key)
            .expect_continue()
            .into_parts();

        assert!(matches!(local, LocalWeather::Fetched(_, _)));
        if let LocalWeather::Fetched(_, ref data) = local {
            assert_eq!(data, &test_response());
        }
    }

    #[test]
    fn location_disabled() {
        let local = LocalWeather::default();
        let api_key = test_api_key();

        let (local, _cmd) = local
            .update(LocalWeatherEvent::LocationEnabled(false), &api_key)
            .expect_continue()
            .into_parts();

        assert!(matches!(local, LocalWeather::LocationDisabled));
    }

    #[test]
    fn location_fetched_triggers_weather_fetch() {
        let local = LocalWeather::default();
        let api_key = test_api_key();

        let lat_lon = Location {
            lat: 33.456_789,
            lon: 112.037_222,
        };

        let (local, mut cmd) = local
            .update(LocalWeatherEvent::LocationFetched(Some(lat_lon)), &api_key)
            .expect_continue()
            .into_parts();

        assert!(matches!(local, LocalWeather::FetchingWeather(_)));

        let mut request = cmd.effects().next().unwrap().expect_http();
        assert_eq!(
            &request.operation,
            &weather::build_request(lat_lon, &test_api_key())
        );

        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(test_response_json().as_bytes())
                    .build(),
            ))
            .unwrap();

        let actual = cmd.events().next().unwrap();
        assert!(matches!(actual, LocalWeatherEvent::WeatherFetched(_)));

        let (local, _cmd) = local
            .update(actual, &api_key)
            .expect_continue()
            .into_parts();

        if let LocalWeather::Fetched(_, ref data) = local {
            assert_eq!(data, &test_response());
            insta::assert_yaml_snapshot!(data);
        } else {
            panic!("Expected Fetched state");
        }
    }
}
