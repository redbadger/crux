use crux_core::render::render;
use crux_core::Command;
use serde::{Deserialize, Serialize};

use crate::location::capability::{get_location, is_location_enabled};
use crate::location::Location;
use crate::weather::client::{WeatherApi, WeatherError};
use crate::weather::model::CurrentResponse;
use crate::{Effect, Model};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum WeatherEvent {
    Show,

    #[serde(skip)]
    LocationEnabled(bool),
    #[serde(skip)]
    LocationFetched(Option<Location>),

    // Events related to fetching weather data
    #[serde(skip)]
    Fetch(Location),
    #[serde(skip)]
    SetWeather(Box<Result<CurrentResponse, WeatherError>>),
    #[serde(skip)]
    FetchFavorites,
    #[serde(skip)]
    SetFavoriteWeather(Box<Result<CurrentResponse, WeatherError>>, Location),
}

pub fn update(event: WeatherEvent, model: &mut Model) -> Command<Effect, WeatherEvent> {
    match event {
        WeatherEvent::Show => is_location_enabled().then_send(WeatherEvent::LocationEnabled),
        WeatherEvent::LocationEnabled(enabled) => {
            model.location_enabled = enabled;
            if enabled {
                get_location().then_send(WeatherEvent::LocationFetched)
            } else {
                Command::done()
            }
        }
        WeatherEvent::LocationFetched(location) => {
            model.last_location.clone_from(&location);
            if let Some(loc) = location {
                update(WeatherEvent::Fetch(loc), model)
            } else {
                Command::done()
            }
        }

        // Internal events related to fetching weather data
        WeatherEvent::Fetch(location) => WeatherApi::fetch(location)
            .then_send(move |result| WeatherEvent::SetWeather(Box::new(result))),
        WeatherEvent::SetWeather(result) => {
            if let Ok(weather_data) = *result {
                model.weather_data = weather_data;
            }

            update(WeatherEvent::FetchFavorites, model).and(render())
        }
        WeatherEvent::FetchFavorites => {
            if model.favorites.is_empty() {
                return Command::done();
            }

            model
                .favorites
                .iter()
                .map(|f| {
                    let location = f.geo.location();

                    WeatherApi::fetch(location).then_send(move |result| {
                        WeatherEvent::SetFavoriteWeather(Box::new(result), location)
                    })
                })
                .collect()
        }
        WeatherEvent::SetFavoriteWeather(result, location) => {
            if let Ok(weather) = *result {
                // Update the weather data for the matching favorite
                if let Some(favorite) = model
                    .favorites
                    .iter_mut()
                    .find(|f| f.geo.location() == location)
                {
                    favorite.current = Some(weather);
                }
            }

            render()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crux_http::protocol::{HttpResponse, HttpResult};

    use crate::{
        favorites::model::Favorite,
        weather::model::{Clouds, Coord, CurrentResponseBuilder, Main, Sys, WeatherData, Wind},
        GeocodingResponse,
    };

    fn test_favorite() -> Favorite {
        Favorite {
            geo: GeocodingResponse {
                name: "Phoenix".to_string(),
                local_names: None,
                lat: 33.456_789,
                lon: -112.037_222,
                country: "US".to_string(),
                state: None,
            },
            current: None,
        }
    }

    fn test_response() -> CurrentResponse {
        CurrentResponseBuilder::default()
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
                sys_type: 1,
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
    fn test_show_triggers_set_weather() {
        let mut model = Model::default();

        // 1. Trigger the Show event
        let event = WeatherEvent::Show;
        let _ = update(event, &mut model);

        // 2. Simulate the Location::is_location_enabled effect (enabled = true)
        let event = WeatherEvent::LocationEnabled(true);
        let _ = update(event, &mut model);

        // 3. Simulate the Location::get_location effect (with a test location)
        let test_location = Location {
            lat: 33.456_789,
            lon: -112.037_222,
        };
        let event = WeatherEvent::LocationFetched(Some(test_location));
        let mut cmd = update(event, &mut model);

        // 4. Resolve the weather HTTP effect
        let mut request = cmd.effects().next().unwrap().expect_http();

        assert_eq!(&request.operation, &WeatherApi::build(test_location));

        // 5. Resolve the HTTP request with a simulated response from the web API
        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(test_response_json().as_bytes())
                    .build(),
            ))
            .unwrap();

        // 6. The next event should be SetWeather
        let actual = cmd.events().next().unwrap();
        assert!(matches!(actual, WeatherEvent::SetWeather(_)));

        // 7. Send the SetWeather event back to the app
        let _ = update(actual.clone(), &mut model);

        // Now check the model in detail
        assert_eq!(model.weather_data, test_response());
    }

    #[test]
    fn test_current_weather_fetch() {
        let mut model = Model::default();

        let lat_lon = Location {
            lat: 33.456_789,
            lon: 112.037_222,
        };
        let event = WeatherEvent::Fetch(lat_lon);

        let mut cmd = update(event, &mut model);

        let mut request = cmd.effects().next().unwrap().expect_http();

        assert_eq!(&request.operation, &WeatherApi::build(lat_lon));

        // Test response handling
        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(test_response_json().as_bytes())
                    .build(),
            ))
            .unwrap();

        let actual = cmd.events().next().unwrap();
        assert!(matches!(actual, WeatherEvent::SetWeather(_)));

        // send the `SetWeather` event back to the app
        let _ = update(actual, &mut model);

        // Now check the model in detail
        assert_eq!(model.weather_data, test_response());
        insta::assert_yaml_snapshot!(model.weather_data);
    }

    #[test]
    fn test_fetch_triggers_favorites_fetch_when_favorites_exist() {
        let mut model = Model::default();

        // Add a favorite
        model.favorites.push(test_favorite());

        let lat_lon = Location {
            lat: 33.456_789,
            lon: 112.037_222,
        };
        let event = WeatherEvent::Fetch(lat_lon);

        // Start the event/effect loop
        let cmd = update(event, &mut model);
        let mut pending_cmds = vec![cmd];

        // Simulate the event/effect loop
        while let Some(mut cmd) = pending_cmds.pop() {
            // Process all effects
            let effects: Vec<_> = cmd.effects().collect();
            for effect in effects {
                if let Effect::Http(mut request) = effect {
                    // Simulate HTTP response
                    request
                        .resolve(HttpResult::Ok(
                            HttpResponse::ok()
                                .body(test_response_json().as_bytes())
                                .build(),
                        ))
                        .unwrap();
                }
            }

            // Process all events
            for event in cmd.events() {
                let next_cmd = update(event.clone(), &mut model);
                pending_cmds.push(next_cmd);
            }
        }

        // After processing, the favorite's weather should be updated
        assert!(model.favorites[0].current.is_some());
        assert_eq!(
            model.favorites[0].current.as_ref().unwrap(),
            &test_response()
        );
    }

    #[test]
    fn test_fetch_favorites_triggers_fetch_for_all_favorites() {
        let mut model = Model::default();

        // Add multiple favorites
        model.favorites.push(test_favorite());
        model.favorites.push(Favorite {
            geo: GeocodingResponse {
                name: "New York".to_string(),
                local_names: None,
                lat: 40.7128,
                lon: -74.0060,
                country: "US".to_string(),
                state: None,
            },
            current: None,
        });

        let event = WeatherEvent::FetchFavorites;
        let mut cmd = update(event, &mut model);

        // Should get HTTP effects for both favorites
        let mut effects = Vec::new();
        while let Some(effect) = cmd.effects().next() {
            effects.push(effect);
        }

        assert_eq!(effects.len(), 2);

        // Verify both favorites are being fetched via HTTP effects
        let mut fetched_locations = Vec::new();
        for effect in effects {
            let _request = effect.expect_http();
            // Extract lat/lon from the request URL or operation
            // This depends on how WeatherApiClient::build_request works
            // For now, just verify we have 2 HTTP effects
            fetched_locations.push(());
        }

        assert_eq!(fetched_locations.len(), 2);
    }
}
