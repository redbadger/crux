use crux_core::render::render;
use crux_core::Command;
use serde::{Deserialize, Serialize};

use crate::config::API_KEY;
use crate::favorites::events::FavoritesEvent;
use crate::location::capability::{get_location, is_location_enabled, LocationResponse};
use crate::weather::model::{CurrentResponse, WEATHER_URL};
use crate::{Effect, Event, Model};
use crux_http::command::Http;

#[derive(Serialize)]
pub struct CurrentQueryString {
    pub lat: String,
    pub lon: String,
    pub units: &'static str,
    pub appid: String,
}

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
    FetchFavorite(f64, f64),
    #[serde(skip)]
    FetchFavorites,
    #[serde(skip)]
    SetWeather(Box<crux_http::Result<crux_http::Response<CurrentResponse>>>),
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
        WeatherEvent::Fetch(lat, long) => Http::get(WEATHER_URL)
            .expect_json()
            .query(&CurrentQueryString {
                lat: lat.to_string(),
                lon: long.to_string(),
                units: "metric",
                appid: API_KEY.clone(),
            })
            .expect("could not serialize query string")
            .build()
            .then_send(|result| Event::Home(Box::new(WeatherEvent::SetWeather(Box::new(result))))),
        WeatherEvent::SetWeather(result) => {
            let cmd = match *result {
                Ok(mut response) => {
                    model.weather_data = response.take_body().unwrap();
                    render()
                }
                Err(_) => render(),
            };

            // If we have favorites, trigger FetchFavorites after setting weather
            if model.favorites.is_empty() {
                cmd
            } else {
                cmd.and(Command::event(Event::Home(Box::new(
                    WeatherEvent::FetchFavorites,
                ))))
            }
        }
        WeatherEvent::FetchFavorites => {
            let cmd = model
                .favorites
                .iter()
                .map(|f| {
                    Command::event(Event::Home(Box::new(WeatherEvent::FetchFavorite(
                        f.geo.lat, f.geo.lon,
                    ))))
                })
                .collect();

            cmd
        }
        WeatherEvent::FetchFavorite(lat, long) => Http::get(WEATHER_URL)
            .expect_json()
            .query(&CurrentQueryString {
                lat: lat.to_string(),
                lon: long.to_string(),
                units: "metric",
                appid: API_KEY.clone(),
            })
            .expect("could not serialize query string")
            .build()
            .then_send(move |result| {
                Event::Home(Box::new(WeatherEvent::SetFavoriteWeather(
                    Box::new(result),
                    lat,
                    long,
                )))
            }),
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
    use crux_http::protocol::{HttpRequest, HttpResponse, HttpResult};

    use crate::{
        favorites::model::Favorite,
        weather::model::current_response::{SAMPLE_CURRENT_RESPONSE, SAMPLE_CURRENT_RESPONSE_JSON},
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
            &HttpRequest::get(WEATHER_URL)
                .query(&CurrentQueryString {
                    lat: test_location.lat.to_string(),
                    lon: test_location.lon.to_string(),
                    units: "metric",
                    appid: API_KEY.clone(),
                })
                .expect("could not serialize query string")
                .build()
        );

        // 5. Resolve the HTTP request with a simulated response from the web API
        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(SAMPLE_CURRENT_RESPONSE_JSON.as_bytes())
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
        let _ = app.update(actual, &mut model, &());

        // Now check the model in detail
        assert_eq!(model.weather_data, *SAMPLE_CURRENT_RESPONSE);
    }

    #[test]
    fn test_current_weather_fetch() {
        let app = App;
        let mut model = Model::default();

        let lat_lng = (33.456_789, 112.037_222);
        let event = Event::Home(Box::new(WeatherEvent::Fetch(lat_lng.0, lat_lng.1)));

        let mut cmd = app.update(event, &mut model, &());

        let mut request = cmd.effects().next().unwrap().expect_http();

        assert_eq!(
            &request.operation,
            &HttpRequest::get(WEATHER_URL)
                .query(&CurrentQueryString {
                    lat: lat_lng.0.to_string(),
                    lon: lat_lng.1.to_string(),
                    units: "metric",
                    appid: API_KEY.clone(),
                })
                .expect("could not serialize query string")
                .build()
        );

        // Test response handling
        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(SAMPLE_CURRENT_RESPONSE_JSON.as_bytes())
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
        assert_eq!(model.weather_data, *SAMPLE_CURRENT_RESPONSE);
        insta::assert_yaml_snapshot!(model.weather_data);
    }

    #[test]
    fn test_fetch_triggers_favorites_fetch_when_favorites_exist() {
        let app = App;
        let mut model = Model::default();

        // Add a favorite
        model.favorites.push(test_favorite());

        let lat_lng = (33.456_789, 112.037_222);
        let event = Event::Home(Box::new(WeatherEvent::Fetch(lat_lng.0, lat_lng.1)));

        let mut cmd = app.update(event, &mut model, &());

        // First request should be for the main location
        let mut request = cmd.effects().next().unwrap().expect_http();

        // Verify the request is for the main location
        assert_eq!(
            &request.operation,
            &HttpRequest::get(WEATHER_URL)
                .query(&CurrentQueryString {
                    lat: lat_lng.0.to_string(),
                    lon: lat_lng.1.to_string(),
                    units: "metric",
                    appid: API_KEY.clone(),
                })
                .expect("could not serialize query string")
                .build()
        );

        // Resolve the request
        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(SAMPLE_CURRENT_RESPONSE_JSON.as_bytes())
                    .build(),
            ))
            .unwrap();

        // Get the next event from the command
        let actual = cmd.events().next().unwrap();
        println!("Got event: {actual:?}"); // Debug print

        // Verify it's a Home event
        let Event::Home(home_event) = &actual else {
            panic!("Expected Home event, got {actual:?}")
        };

        // Verify it's a SetWeather event
        match **home_event {
            WeatherEvent::SetWeather(_) => (),
            _ => panic!("Expected SetWeather event, got {home_event:?}"),
        }

        // Send SetWeather back to app
        let mut cmd = app.update(actual, &mut model, &());

        // Get the next event (should be FetchFavorites)
        let actual = cmd.events().next().unwrap();
        println!("Got event after SetWeather: {actual:?}"); // Debug print

        // Verify it's a FetchFavorites event
        match &actual {
            Event::Home(event) => {
                assert!(
                    matches!(**event, WeatherEvent::FetchFavorites),
                    "Expected FetchFavorites event, got {event:?}"
                );
            }
            _ => panic!("Expected Home event, got {actual:?}"),
        }

        // Send FetchFavorites back to app
        let mut cmd = app.update(actual, &mut model, &());

        // Get the next event (should be FetchFavorite for our favorite)
        let actual = cmd.events().next().unwrap();
        println!("Got event after FetchFavorites: {actual:?}"); // Debug print

        // Verify it's a FetchFavorite event for our favorite
        match &actual {
            Event::Home(event) => {
                if let WeatherEvent::FetchFavorite(lat, lon) = **event {
                    assert_eq!(lat.to_bits(), 33.456_789f64.to_bits());
                    assert_eq!(lon.to_bits(), (-112.037_222f64).to_bits());
                } else {
                    panic!("Expected FetchFavorite event, got {event:?}");
                }
            }
            _ => panic!("Expected Home event, got {actual:?}"),
        }
    }

    #[test]
    fn test_fetch_favorite_updates_favorite_weather() {
        let app = App;
        let mut model = Model::default();

        // Add a favorite
        model.favorites.push(test_favorite());

        let lat_lng = (33.456_789, -112.037_222);
        let event = Event::Home(Box::new(WeatherEvent::FetchFavorite(lat_lng.0, lat_lng.1)));

        let mut cmd = app.update(event, &mut model, &());

        // Should get a request for the favorite's weather
        let mut request = cmd.effects().next().unwrap().expect_http();
        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(SAMPLE_CURRENT_RESPONSE_JSON.as_bytes())
                    .build(),
            ))
            .unwrap();

        // Next event should be SetFavoriteWeather
        let actual = cmd.events().next().unwrap();
        if let Event::Home(event) = &actual {
            if let WeatherEvent::SetFavoriteWeather(_, lat, lon) = **event {
                assert_eq!(lat.to_bits(), lat_lng.0.to_bits());
                assert_eq!(lon.to_bits(), lat_lng.1.to_bits());
            } else {
                panic!("Expected SetFavoriteWeather event")
            }
        } else {
            panic!("Expected Home event")
        }

        // Send SetFavoriteWeather back to app
        let _ = app.update(actual, &mut model, &());

        // Verify the favorite's weather was updated
        assert!(model.favorites[0].current.is_some());
        assert_eq!(
            model.favorites[0].current.as_ref().unwrap(),
            &*SAMPLE_CURRENT_RESPONSE
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

        // Should get FetchFavorite events for both favorites
        let mut events = Vec::new();
        while let Some(event) = cmd.events().next() {
            events.push(event);
        }

        assert_eq!(events.len(), 2);

        // Verify both favorites are being fetched
        let mut fetched_locations = Vec::new();
        for event in events {
            if let Event::Home(event) = event {
                if let WeatherEvent::FetchFavorite(lat, lon) = *event {
                    fetched_locations.push((lat, lon));
                }
            }
        }

        assert!(fetched_locations.contains(&(33.456_789, -112.037_222)));
        assert!(fetched_locations.contains(&(40.7128, -74.0060)));
    }
}
