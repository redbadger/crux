use crux_core::{render::render, Command};
use crux_http::command::Http;
use serde::{Deserialize, Serialize};

use crate::workflows::home::HomeEvent;
use crate::{CurrentResponse, Effect, Event, Model};

const WEATHER_URL: &str = "https://api.openweathermap.org/data/2.5/weather";
const API_KEY: &str = "42005d273a8a49c88a8173878232508";

#[derive(Serialize)]
pub struct CurrentQueryString {
    pub lat: String,
    pub lon: String,
    pub appid: &'static str,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CurrentWeatherEvent {
    #[serde(skip)]
    Fetch(f64, f64),
    #[serde(skip)]
    SetWeather(crux_http::Result<crux_http::Response<CurrentResponse>>),
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
            .then_send(|result| Event::CurrentWeather(CurrentWeatherEvent::SetWeather(result))),
        CurrentWeatherEvent::SetWeather(result) => {
            model.weather_data = result.unwrap().take_body().unwrap();
            render()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crux_core::App as _;
    use crux_http::{
        protocol::{HttpRequest, HttpResponse, HttpResult},
        testing::ResponseBuilder,
    };

    #[test]
    fn test_current_weather_fetch() {
        let lat_lng = (33.456789, -112.037222);
        let event = CurrentWeatherEvent::Fetch(lat_lng.0, lat_lng.1);

        let mut cmd = update(event, &mut Model::default());

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
                    .body(
                        r#"{
                            "main": {
                                "temp": 20.0,
                                "feels_like": 18.0,
                                "temp_min": 18.0,
                                "temp_max": 22.0,
                                "pressure": 1013,
                                "humidity": 50
                            },
                            "coord": {
                                "lat": 33.456789,
                                "lon": -112.037222
                            },
                            "weather": [{
                                "id": 800,
                                "main": "Clear",
                                "description": "clear sky",
                                "icon": "01d"
                            }],
                            "base": "",
                            "visibility": 10000,
                            "wind": {
                                "speed": 4.1,
                                "deg": 280,
                                "gust": 5.2
                            },
                            "clouds": {
                                "all": 0
                            },
                            "dt": 1716216000,
                            "sys": {
                                "id": 1,
                                "country": "US",
                                "type": 1,
                                "sunrise": 1716216000,
                                "sunset": 1716216000
                            },
                            "timezone": 1,
                            "id": 1,
                            "name": "Phoenix",
                            "cod": 200
                        }"#,
                    )
                    .build(),
            ))
            .unwrap();

        let actual = cmd.events().next().unwrap();
        assert!(matches!(
            actual,
            Event::CurrentWeather(CurrentWeatherEvent::SetWeather(_))
        ));
    }
}
