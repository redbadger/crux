use crux_core::{Command, render::render};

use crate::effects::http::weather::{self as weather_api, WeatherError};
use crate::effects::http::weather::model::current_response::CurrentWeatherResponse;
use crate::effects::location::Location;
use crate::effects::location::command::get_location;
use crate::model::ApiKey;
use crate::model::outcome::{Outcome, Started};

#[derive(Clone, Debug, PartialEq)]
pub enum LocalWeatherEvent {
    LocationEnabled(bool),
    LocationFetched(Option<Location>),
    WeatherFetched(Box<Result<CurrentWeatherResponse, WeatherError>>),
    Retry,
}

#[derive(Debug)]
pub(crate) enum LocalWeatherTransition {
    Unauthorized,
}

#[derive(Debug, Clone, Default)]
pub enum LocalWeather {
    #[default]
    CheckingPermission,
    LocationDisabled,
    FetchingLocation,
    FetchingWeather(Location),
    Fetched(Location, Box<CurrentWeatherResponse>),
    Failed(Location),
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
            LocalWeatherEvent::Retry => {
                let Started { state, command } = Self::start();
                Outcome::continuing(state, command)
            }
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
                        Outcome::continuing(
                            Self::Fetched(location, Box::new(weather_data)),
                            render(),
                        )
                    }
                    Err(WeatherError::Unauthorized) => {
                        tracing::warn!("weather API returned unauthorized");
                        Outcome::complete(LocalWeatherTransition::Unauthorized, render())
                    }
                    Err(ref e) => {
                        tracing::warn!("fetching weather failed: {e:?}");
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
        model::{ApiKey, Effect},
    };

    use crate::effects::http::weather;
    use crate::effects::http::weather::model::{
        current_response::{CurrentWeatherResponse, CurrentWeatherResponseBuilder, Main, Sys},
        response_elements::{Clouds, Coord, WeatherData, Wind},
    };
    use super::*;

    const TEST_API_KEY: &str = "test_api_key";

    fn api_key() -> ApiKey {
        TEST_API_KEY.to_string().into()
    }

    fn phoenix_location() -> Location {
        Location {
            lat: 33.456_789,
            lon: -112.037_222,
        }
    }

    fn phoenix_weather_response() -> CurrentWeatherResponse {
        let location = phoenix_location();
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
                lat: location.lat,
                lon: location.lon,
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

    fn phoenix_weather_json() -> String {
        serde_json::to_string(&phoenix_weather_response()).unwrap()
    }

    /// Drives the state machine from `FetchingLocation` through to `FetchingWeather`,
    /// resolving location and returning the state + command ready for a weather response.
    fn drive_to_fetching_weather() -> (LocalWeather, Command<Effect, LocalWeatherEvent>) {
        let local = LocalWeather::default();
        let key = api_key();

        let (local, mut cmd) = local
            .update(LocalWeatherEvent::LocationEnabled(true), &key)
            .expect_continue()
            .into_parts();

        let mut location_effect = cmd.expect_one_effect().expect_location();
        location_effect
            .resolve(LocationResult::Location(Some(phoenix_location())))
            .expect("to resolve");

        let event = cmd.expect_one_event();
        local.update(event, &key).expect_continue().into_parts()
    }

    #[test]
    fn location_enabled_fetches_location() {
        let local = LocalWeather::default();

        let (local, mut cmd) = local
            .update(LocalWeatherEvent::LocationEnabled(true), &api_key())
            .expect_continue()
            .into_parts();

        assert!(matches!(local, LocalWeather::FetchingLocation));

        let location_effect = cmd.expect_one_effect().expect_location();
        assert_eq!(location_effect.operation, LocationOperation::GetLocation);
    }

    #[test]
    fn location_disabled() {
        let local = LocalWeather::default();

        let (local, _cmd) = local
            .update(LocalWeatherEvent::LocationEnabled(false), &api_key())
            .expect_continue()
            .into_parts();

        assert!(matches!(local, LocalWeather::LocationDisabled));
    }

    #[test]
    fn location_fetched_triggers_weather_fetch() {
        let local = LocalWeather::default();
        let location = phoenix_location();

        let (local, mut cmd) = local
            .update(
                LocalWeatherEvent::LocationFetched(Some(location)),
                &api_key(),
            )
            .expect_continue()
            .into_parts();

        assert!(matches!(local, LocalWeather::FetchingWeather(_)));

        let request = cmd.effects().next().unwrap().expect_http();
        assert_eq!(
            &request.operation,
            &weather::build_request(location, &api_key())
        );
    }

    #[test]
    fn location_fetched_none_disables() {
        let local = LocalWeather::default();

        let (local, _cmd) = local
            .update(LocalWeatherEvent::LocationFetched(None), &api_key())
            .expect_continue()
            .into_parts();

        assert!(matches!(local, LocalWeather::LocationDisabled));
    }

    #[test]
    fn weather_fetched_stores_data() {
        let (local, mut cmd) = drive_to_fetching_weather();
        assert!(matches!(local, LocalWeather::FetchingWeather(_)));

        let mut request = cmd.expect_one_effect().expect_http();
        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(phoenix_weather_json().as_bytes())
                    .build(),
            ))
            .unwrap();

        let event = cmd.expect_one_event();
        let (local, _cmd) = local
            .update(event, &api_key())
            .expect_continue()
            .into_parts();

        let LocalWeather::Fetched(loc, ref data) = local else {
            panic!("Expected Fetched state, got {local:?}");
        };
        assert_eq!(loc, phoenix_location());
        assert_eq!(data.as_ref(), &phoenix_weather_response());
        insta::assert_yaml_snapshot!(data.as_ref());
    }

    #[test]
    fn weather_unauthorized_completes_with_transition() {
        let (local, mut cmd) = drive_to_fetching_weather();

        let mut request = cmd.expect_one_effect().expect_http();
        request
            .resolve(HttpResult::Ok(
                HttpResponse::status(401).body(b"Unauthorized").build(),
            ))
            .unwrap();

        let event = cmd.expect_one_event();
        let (transition, _cmd) = local
            .update(event, &api_key())
            .expect_complete()
            .into_parts();

        assert!(matches!(transition, LocalWeatherTransition::Unauthorized));
    }

    #[test]
    fn weather_network_error_transitions_to_failed() {
        let (local, mut cmd) = drive_to_fetching_weather();

        let mut request = cmd.expect_one_effect().expect_http();
        request
            .resolve(HttpResult::Err(
                crux_http::HttpError::Url("connection refused".into()),
            ))
            .unwrap();

        let event = cmd.expect_one_event();
        let (local, _cmd) = local
            .update(event, &api_key())
            .expect_continue()
            .into_parts();

        assert!(matches!(local, LocalWeather::Failed(_)));
    }

    #[test]
    fn retry_restarts_from_checking_permission() {
        let local = LocalWeather::LocationDisabled;

        let (local, mut cmd) = local
            .update(LocalWeatherEvent::Retry, &api_key())
            .expect_continue()
            .into_parts();

        assert!(matches!(local, LocalWeather::CheckingPermission));

        let location_effect = cmd.expect_one_effect().expect_location();
        assert_eq!(
            location_effect.operation,
            LocationOperation::IsLocationEnabled
        );
    }
}
