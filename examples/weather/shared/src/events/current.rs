use crux_core::{render::render, Command};
use crux_http::command::Http;
use serde::{Deserialize, Serialize};

use crate::{CurrentResponse, Effect, Event, Model};

pub const WEATHER_URL: &str = "https://api.openweathermap.org/data/2.5/weather";
pub const API_KEY: &str = "42005d273a8a49c88a8173878232508";

#[derive(Serialize)]
pub struct CurrentQueryString {
    pub lat: String,
    pub lon: String,
    pub appid: &'static str,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CurrentWeatherEvent {
    Fetch(f64, f64),
    SetWeather(Box<crux_http::Result<crux_http::Response<CurrentResponse>>>),
}

pub fn update(event: CurrentWeatherEvent, model: &mut Model) -> Command<Effect, Event> {
    match event {
        CurrentWeatherEvent::Fetch(lat, long) => Http::get(WEATHER_URL)
            .expect_json()
            .query(&CurrentQueryString {
                lat: lat.to_string(),
                lon: long.to_string(),
                appid: API_KEY,
            })
            .expect("could not serialize query string")
            .build()
            .then_send(|result| {
                Event::CurrentWeather(Box::new(CurrentWeatherEvent::SetWeather(Box::new(result))))
            }),
        CurrentWeatherEvent::SetWeather(result) => match *result {
            Ok(mut response) => {
                model.weather_data = response.take_body().unwrap();
                render()
            }
            Err(_) => render(),
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::{App, SAMPLE_CURRENT_RESPONSE, SAMPLE_CURRENT_RESPONSE_JSON};

    use super::*;
    use crux_core::{assert_effect, App as _};
    use crux_http::protocol::{HttpRequest, HttpResponse, HttpResult};

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
}
