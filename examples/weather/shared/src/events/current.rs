use crux_core::{render::render, Command};
use crux_http::command::Http;
use serde::{Deserialize, Serialize};

use crate::{CurrentResponse, Effect, Event, Model};

pub const WEATHER_URL: &str = "https://api.openweathermap.org/data/2.5/weather";
pub const API_KEY: &str = "4e72eedd054f22249d785de2ac3ab627";

#[derive(Serialize)]
pub struct CurrentQueryString {
    pub lat: String,
    pub lon: String,
    pub units: &'static str,
    pub appid: &'static str,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CurrentWeatherEvent {
    Fetch(f64, f64),
    FetchFavorite(f64, f64),
    FetchFavorites,
    SetWeather(Box<crux_http::Result<crux_http::Response<CurrentResponse>>>),
    SetFavoriteWeather(
        Box<crux_http::Result<crux_http::Response<CurrentResponse>>>,
        f64,
        f64,
    ),
}

pub fn update(event: CurrentWeatherEvent, model: &mut Model) -> Command<Effect, Event> {
    match event {
        CurrentWeatherEvent::Fetch(lat, long) => Http::get(WEATHER_URL)
            .expect_json()
            .query(&CurrentQueryString {
                lat: lat.to_string(),
                lon: long.to_string(),
                units: "metric",
                appid: API_KEY,
            })
            .expect("could not serialize query string")
            .build()
            .then_send(|result| {
                Event::CurrentWeather(Box::new(CurrentWeatherEvent::SetWeather(Box::new(result))))
            }),
        CurrentWeatherEvent::SetWeather(result) => {
            let cmd = match *result {
                Ok(mut response) => {
                    model.weather_data = response.take_body().unwrap();
                    render()
                }
                Err(_) => render(),
            };

            // If we have favorites, trigger FetchFavorites after setting weather
            if !model.favorites.is_empty() {
                cmd.and(Command::event(Event::CurrentWeather(Box::new(
                    CurrentWeatherEvent::FetchFavorites,
                ))))
            } else {
                cmd
            }
        }
        CurrentWeatherEvent::FetchFavorites => {
            // Create a sequence of commands to fetch weather for each favorite
            let mut cmd = Command::done();

            for favorite in &model.favorites {
                cmd = cmd.and(Command::event(Event::CurrentWeather(Box::new(
                    CurrentWeatherEvent::FetchFavorite(favorite.geo.lat, favorite.geo.lon),
                ))));
            }

            cmd
        }
        CurrentWeatherEvent::FetchFavorite(lat, long) => {
            let lat = lat;
            let long = long;
            Http::get(WEATHER_URL)
                .expect_json()
                .query(&CurrentQueryString {
                    lat: lat.to_string(),
                    lon: long.to_string(),
                    units: "metric",
                    appid: API_KEY,
                })
                .expect("could not serialize query string")
                .build()
                .then_send(move |result| {
                    Event::CurrentWeather(Box::new(CurrentWeatherEvent::SetFavoriteWeather(
                        Box::new(result),
                        lat,
                        long,
                    )))
                })
        }
        CurrentWeatherEvent::SetFavoriteWeather(result, lat, long) => match *result {
            Ok(mut response) => {
                let weather = response.take_body().unwrap();
                // Update the weather data for the matching favorite
                if let Some(favorite) = model
                    .favorites
                    .iter_mut()
                    .find(|f| f.geo.lat == lat && f.geo.lon == long)
                {
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
    use crate::{
        workflows::favorites::Favorite, App, GeocodingResponse, Model, SAMPLE_CURRENT_RESPONSE,
        SAMPLE_CURRENT_RESPONSE_JSON,
    };

    use super::*;
    use crux_core::{assert_effect, App as _};
    use crux_http::protocol::{HttpRequest, HttpResponse, HttpResult};

    // Helper to create a test favorite
    fn test_favorite() -> Favorite {
        Favorite {
            geo: GeocodingResponse {
                name: "Phoenix".to_string(),
                local_names: None,
                lat: 33.456789,
                lon: -112.037222,
                country: "US".to_string(),
                state: None,
            },
            current: None,
        }
    }

    #[test]
    fn test_current_weather_fetch() {
        let app = App::default();
        let mut model = Model::default();

        let lat_lng = (33.456789, -112.037222);
        let event =
            Event::CurrentWeather(Box::new(CurrentWeatherEvent::Fetch(lat_lng.0, lat_lng.1)));

        let mut cmd = app.update(event, &mut model, &mut ());

        let mut request = cmd.effects().next().unwrap().expect_http();

        assert_eq!(
            &request.operation,
            &HttpRequest::get(WEATHER_URL)
                .query(&CurrentQueryString {
                    lat: lat_lng.0.to_string(),
                    lon: lat_lng.1.to_string(),
                    units: "metric",
                    appid: API_KEY,
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
        if let Event::CurrentWeather(event) = &actual {
            assert!(matches!(**event, CurrentWeatherEvent::SetWeather(_)))
        } else {
            panic!("Expected CurrentWeather event")
        }

        // send the `SetWeather` event back to the app
        let mut cmd = app.update(actual, &mut model, &mut ());
        assert_effect!(cmd, Effect::Render(_));
        // Now check the model in detail
        assert_eq!(model.weather_data, *SAMPLE_CURRENT_RESPONSE);
        insta::assert_yaml_snapshot!(model.weather_data);
    }

    #[test]
    fn test_fetch_triggers_favorites_fetch_when_favorites_exist() {
        let app = App::default();
        let mut model = Model::default();

        // Add a favorite
        model.favorites.push(test_favorite());

        let lat_lng = (33.456789, -112.037222);
        let event =
            Event::CurrentWeather(Box::new(CurrentWeatherEvent::Fetch(lat_lng.0, lat_lng.1)));

        let mut cmd = app.update(event, &mut model, &mut ());

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
                    appid: API_KEY,
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
        println!("Got event: {:?}", actual); // Debug print

        // Verify it's a CurrentWeather event
        let current_weather_event = match &actual {
            Event::CurrentWeather(event) => event,
            _ => panic!("Expected CurrentWeather event, got {:?}", actual),
        };

        // Verify it's a SetWeather event
        match **current_weather_event {
            CurrentWeatherEvent::SetWeather(_) => (),
            _ => panic!("Expected SetWeather event, got {:?}", current_weather_event),
        }

        // Send SetWeather back to app
        let mut cmd = app.update(actual, &mut model, &mut ());

        // Get the next event (should be FetchFavorites)
        let actual = cmd.events().next().unwrap();
        println!("Got event after SetWeather: {:?}", actual); // Debug print

        // Verify it's a FetchFavorites event
        match &actual {
            Event::CurrentWeather(event) => {
                assert!(
                    matches!(**event, CurrentWeatherEvent::FetchFavorites),
                    "Expected FetchFavorites event, got {:?}",
                    event
                );
            }
            _ => panic!("Expected CurrentWeather event, got {:?}", actual),
        }

        // Send FetchFavorites back to app
        let mut cmd = app.update(actual, &mut model, &mut ());

        // Get the next event (should be FetchFavorite for our favorite)
        let actual = cmd.events().next().unwrap();
        println!("Got event after FetchFavorites: {:?}", actual); // Debug print

        // Verify it's a FetchFavorite event for our favorite
        match &actual {
            Event::CurrentWeather(event) => {
                if let CurrentWeatherEvent::FetchFavorite(lat, lon) = **event {
                    assert_eq!(lat, 33.456789);
                    assert_eq!(lon, -112.037222);
                } else {
                    panic!("Expected FetchFavorite event, got {:?}", event);
                }
            }
            _ => panic!("Expected CurrentWeather event, got {:?}", actual),
        }
    }

    #[test]
    fn test_fetch_favorite_updates_favorite_weather() {
        let app = App::default();
        let mut model = Model::default();

        // Add a favorite
        model.favorites.push(test_favorite());

        let lat_lng = (33.456789, -112.037222);
        let event = Event::CurrentWeather(Box::new(CurrentWeatherEvent::FetchFavorite(
            lat_lng.0, lat_lng.1,
        )));

        let mut cmd = app.update(event, &mut model, &mut ());

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
        if let Event::CurrentWeather(event) = &actual {
            if let CurrentWeatherEvent::SetFavoriteWeather(_, lat, lon) = **event {
                assert_eq!(lat, lat_lng.0);
                assert_eq!(lon, lat_lng.1);
            } else {
                panic!("Expected SetFavoriteWeather event")
            }
        } else {
            panic!("Expected CurrentWeather event")
        }

        // Send SetFavoriteWeather back to app
        let mut cmd = app.update(actual, &mut model, &mut ());
        assert_effect!(cmd, Effect::Render(_));

        // Verify the favorite's weather was updated
        assert!(model.favorites[0].current.is_some());
        assert_eq!(
            model.favorites[0].current.as_ref().unwrap(),
            &*SAMPLE_CURRENT_RESPONSE
        );
    }

    #[test]
    fn test_fetch_favorites_triggers_fetch_for_all_favorites() {
        let app = App::default();
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

        let event = Event::CurrentWeather(Box::new(CurrentWeatherEvent::FetchFavorites));
        let mut cmd = app.update(event, &mut model, &mut ());

        // Should get FetchFavorite events for both favorites
        let mut events = Vec::new();
        while let Some(event) = cmd.events().next() {
            events.push(event);
        }

        assert_eq!(events.len(), 2);

        // Verify both favorites are being fetched
        let mut fetched_locations = Vec::new();
        for event in events {
            if let Event::CurrentWeather(event) = event {
                if let CurrentWeatherEvent::FetchFavorite(lat, lon) = *event {
                    fetched_locations.push((lat, lon));
                }
            }
        }

        assert!(fetched_locations.contains(&(33.456789, -112.037222)));
        assert!(fetched_locations.contains(&(40.7128, -74.0060)));
    }
}
