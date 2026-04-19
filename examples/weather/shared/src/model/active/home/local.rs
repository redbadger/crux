//! The local-weather state machine shown on the home screen.
//!
//! Walks through the steps needed to display weather at the user's current
//! location: check whether location services are enabled, fetch the current
//! coordinates, and request the weather for those coordinates. Each step
//! can fail or be short-circuited, so the machine is modelled as an enum
//! of mutually-exclusive states rather than a struct with optional fields.

use crux_core::{Command, render::render};

use crate::effects::http::weather::model::current_response::CurrentWeatherResponse;
use crate::effects::http::weather::{self as weather_api, WeatherError};
use crate::effects::location::Location;
use crate::effects::location::command::get_location;
use crate::model::ApiKey;
use crate::model::outcome::{Outcome, Started};

// ANCHOR: event
/// Events emitted as location permission, location fetch, and weather fetch
/// resolve — plus an explicit retry from the UI.
#[derive(Clone, Debug, PartialEq)]
pub enum LocalWeatherEvent {
    /// The shell reported whether location services are enabled.
    LocationEnabled(bool),
    /// The shell returned the current coordinates, or `None` if it couldn't
    /// determine them.
    LocationFetched(Option<Location>),
    /// The weather API responded with current conditions, or an error.
    WeatherFetched(Box<Result<CurrentWeatherResponse, WeatherError>>),
    /// The user tapped "retry" after a disabled or failed state.
    Retry,
}
// ANCHOR_END: event

// ANCHOR: transition
/// The exits from the local-weather state machine.
///
/// Only one today: the weather API rejected our key, so the parent should
/// bubble up to a reset/onboarding flow.
#[derive(Debug)]
pub(crate) enum LocalWeatherTransition {
    /// The weather API returned 401; the API key needs re-entry.
    Unauthorized,
}
// ANCHOR_END: transition

// ANCHOR: state
/// The state of the local-weather workflow.
///
/// The machine progresses through these states as events resolve:
/// `CheckingPermission` → `FetchingLocation` → `FetchingWeather` → `Fetched`.
/// Either permission or location can short-circuit to `LocationDisabled`,
/// and a failed weather fetch lands in `Failed`. All non-terminal states
/// accept `Retry` to restart from the beginning.
#[derive(Debug, Clone, Default)]
pub enum LocalWeather {
    /// Initial state: asking the shell whether location services are on.
    #[default]
    CheckingPermission,
    /// Location services are off or the user denied them; the UI shows a
    /// "location disabled" panel with a retry button.
    LocationDisabled,
    /// Location services are on; waiting for the shell to return the
    /// current coordinates.
    FetchingLocation,
    /// We have coordinates; waiting for the weather API response.
    FetchingWeather(Location),
    /// We have current weather for the user's location — terminal happy
    /// path until a `Retry`.
    Fetched(Location, Box<CurrentWeatherResponse>),
    /// Weather fetch failed for reasons other than unauthorized (network,
    /// malformed response). The UI shows an error with a retry button.
    Failed(Location),
}
// ANCHOR_END: state

impl LocalWeather {
    // ANCHOR: start
    /// Starts the state machine in `CheckingPermission` and asks the shell
    /// whether location services are enabled.
    pub(crate) fn start() -> Started<Self, LocalWeatherEvent> {
        tracing::debug!("checking location permissions");
        let cmd = crate::effects::location::command::is_location_enabled()
            .then_send(LocalWeatherEvent::LocationEnabled);
        Started::new(Self::CheckingPermission, cmd)
    }
    // ANCHOR_END: start

    // ANCHOR: update
    /// Advances the state machine on an event, using `api_key` to authorise
    /// the weather API call when needed.
    ///
    /// - `LocationEnabled(true)` → fetch location → `FetchingLocation`.
    /// - `LocationEnabled(false)` or `LocationFetched(None)` →
    ///   `LocationDisabled` with a render.
    /// - `LocationFetched(Some(_))` → request weather → `FetchingWeather`.
    /// - `WeatherFetched(Ok)` → `Fetched` with the response.
    /// - `WeatherFetched(Err(Unauthorized))` → `Complete` with
    ///   [`LocalWeatherTransition::Unauthorized`].
    /// - `WeatherFetched(Err(_))` → `Failed` (network or parse errors).
    /// - `Retry` → restart via [`Self::start`].
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
    // ANCHOR_END: update
}

#[cfg(test)]
mod tests {
    use crux_http::protocol::{HttpResponse, HttpResult};

    use crate::{
        effects::location::{Location, LocationOperation, LocationResult},
        model::{ApiKey, Effect},
    };

    use super::*;
    use crate::effects::http::weather;
    use crate::effects::http::weather::model::{
        current_response::{CurrentWeatherResponse, CurrentWeatherResponseBuilder, Main, Sys},
        response_elements::{Clouds, Coord, WeatherData, Wind},
    };

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

    // ANCHOR: drive_helper
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
    // ANCHOR_END: drive_helper

    // ANCHOR: simple_test
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
    // ANCHOR_END: simple_test

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

    // ANCHOR: full_test
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
    // ANCHOR_END: full_test

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
            .resolve(HttpResult::Err(crux_http::HttpError::Url(
                "connection refused".into(),
            )))
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
