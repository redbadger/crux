use crux_core::render::render;
use crux_core::Command;
use serde::{Deserialize, Serialize};

use crate::favorites::events::FavoritesEvent;
use crate::location::capability::{get_location, is_location_enabled, LocationResponse};
use crate::weather::client::WeatherApi;
use crate::weather::model::CurrentResponse;
use crate::{Effect, Event, Model};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum WeatherEvent {
    Show,

    #[serde(skip)]
    LocationEnabled(bool),
    #[serde(skip)]
    LocationFetched(Option<LocationResponse>),

    // Events related to fetching weather data
    #[serde(skip)]
    Fetch(f64, f64),
    #[serde(skip)]
    SetWeather(Box<crux_http::Result<crux_http::Response<CurrentResponse>>>),
    #[serde(skip)]
    FetchFavorites,
    #[serde(skip)]
    SetFavoriteWeather(
        Box<crux_http::Result<crux_http::Response<CurrentResponse>>>,
        f64,
        f64,
    ),
}

pub fn update(event: WeatherEvent, model: &mut Model) -> Command<Effect, Event> {
    match event {
        WeatherEvent::Show => Command::event(Event::Favorites(Box::new(FavoritesEvent::Restore)))
            .then(
                is_location_enabled().then_send(|result| {
                    Event::Home(Box::new(WeatherEvent::LocationEnabled(result)))
                }),
            ),
        WeatherEvent::LocationEnabled(enabled) => {
            model.location_enabled = enabled;
            if enabled {
                get_location().then_send(|result| {
                    Event::Home(Box::new(WeatherEvent::LocationFetched(result)))
                })
            } else {
                Command::done()
            }
        }
        WeatherEvent::LocationFetched(location) => {
            model.last_location.clone_from(&location);
            if let Some(loc) = location {
                update(WeatherEvent::Fetch(loc.lat, loc.lon), model)
            } else {
                Command::done()
            }
        }

        // Internal events related to fetching weather data
        WeatherEvent::Fetch(lat, long) => WeatherApi::fetch(lat, long).then_send(move |result| {
            Event::Home(Box::new(WeatherEvent::SetWeather(Box::new(result))))
        }),
        WeatherEvent::SetWeather(result) => {
            let cmd = match *result {
                Ok(mut response) => {
                    model.weather_data = response.take_body().unwrap();
                    render()
                }
                Err(_) => render(),
            };

            if model.favorites.is_empty() {
                cmd
            } else {
                cmd.then(Command::event(Event::Home(Box::new(
                    WeatherEvent::FetchFavorites,
                ))))
            }
        }
        WeatherEvent::FetchFavorites => {
            if model.favorites.is_empty() {
                Command::done()
            } else {
                let cmds = model.favorites.iter().map(|f| {
                    let lat = f.geo.lat;
                    let lon = f.geo.lon;
                    WeatherApi::fetch(lat, lon).then_send(move |result| {
                        Event::Home(Box::new(WeatherEvent::SetFavoriteWeather(
                            Box::new(result),
                            lat,
                            lon,
                        )))
                    })
                });

                cmds.collect()
            }
        }
        WeatherEvent::SetFavoriteWeather(result, lat, long) => match *result {
            Ok(mut response) => {
                let weather = response.take_body().unwrap();
                // Update the weather data for the matching favorite
                if let Some(favorite) = model.favorites.iter_mut().find(|f| {
                    f.geo.lat.to_bits() == lat.to_bits() && f.geo.lon.to_bits() == long.to_bits()
                }) {
                    favorite.current = Some(weather);
                }
                render()
            }
            Err(_) => render(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crux_core::App as _;
    use crux_http::protocol::{HttpResponse, HttpResult};

    use crate::{
        favorites::model::Favorite,
        weather::model::{Clouds, Coord, CurrentResponseBuilder, Main, Sys, WeatherData, Wind},
        App, GeocodingResponse,
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
        let app = App;
        let mut model = Model::default();

        // 1. Trigger the Show event
        let event = Event::Home(Box::new(WeatherEvent::Show));
        let _ = app.update(event, &mut model, &());

        // 2. Simulate the Location::is_location_enabled effect (enabled = true)
        let event = Event::Home(Box::new(WeatherEvent::LocationEnabled(true)));
        let _ = app.update(event, &mut model, &());

        // 3. Simulate the Location::get_location effect (with a test location)
        let test_location = LocationResponse {
            lat: 33.456_789,
            lon: -112.037_222,
        };
        let event = Event::Home(Box::new(WeatherEvent::LocationFetched(Some(
            test_location.clone(),
        ))));
        let mut cmd = app.update(event, &mut model, &());

        // 4. Resolve the weather HTTP effect
        let mut request = cmd.effects().next().unwrap().expect_http();

        assert_eq!(
            &request.operation,
            &WeatherApi::build(test_location.lat, test_location.lon)
        );

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
        if let Event::Home(event) = &actual {
            assert!(matches!(**event, WeatherEvent::SetWeather(_)));
        } else {
            panic!("Expected Home event")
        }

        // 7. Send the SetWeather event back to the app
        let _ = app.update(actual.clone(), &mut model, &());

        // Now check the model in detail
        assert_eq!(model.weather_data, test_response());
    }

    #[test]
    fn test_current_weather_fetch() {
        let app = App;
        let mut model = Model::default();

        let lat_lon = (33.456_789, 112.037_222);
        let event = Event::Home(Box::new(WeatherEvent::Fetch(lat_lon.0, lat_lon.1)));

        let mut cmd = app.update(event, &mut model, &());

        let mut request = cmd.effects().next().unwrap().expect_http();

        assert_eq!(&request.operation, &WeatherApi::build(lat_lon.0, lat_lon.1));

        // Test response handling
        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(test_response_json().as_bytes())
                    .build(),
            ))
            .unwrap();

        let actual = cmd.events().next().unwrap();
        if let Event::Home(event) = &actual {
            assert!(matches!(**event, WeatherEvent::SetWeather(_)));
        } else {
            panic!("Expected Home event")
        }

        // send the `SetWeather` event back to the app
        let _ = app.update(actual, &mut model, &());

        // Now check the model in detail
        assert_eq!(model.weather_data, test_response());
        insta::assert_yaml_snapshot!(model.weather_data);
    }

    #[test]
    fn test_fetch_triggers_favorites_fetch_when_favorites_exist() {
        let app = App;
        let mut model = Model::default();

        // Add a favorite
        model.favorites.push(test_favorite());

        let lat_lon = (33.456_789, 112.037_222);
        let event = Event::Home(Box::new(WeatherEvent::Fetch(lat_lon.0, lat_lon.1)));

        // Start the event/effect loop
        let cmd = app.update(event, &mut model, &());
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
                let next_cmd = app.update(event.clone(), &mut model, &());
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
        let app = App;
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

        let event = Event::Home(Box::new(WeatherEvent::FetchFavorites));
        let mut cmd = app.update(event, &mut model, &());

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
