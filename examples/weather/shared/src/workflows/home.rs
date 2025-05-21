use crux_core::Command;
use crux_http::command::Http;
use serde::{Deserialize, Serialize};

use crate::{Effect, Event, Model};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum HomeEvent {
    Show(f64, f64),
}

// Query string example from https://openweathermap.org/current
#[derive(Serialize)]
struct CurrentQueryString {
    lat: String,
    lon: String,
    appid: &'static str,
}

const WEATHER_URL: &str = "https://api.openweathermap.org/data/2.5/weather";
const API_KEY: &str = "42005d273a8a49c88a8173878232508";

pub fn update(event: HomeEvent, _model: &mut Model) -> Command<Effect, Event> {
    match event {
        HomeEvent::Show(lat, long) => Http::get(WEATHER_URL)
            .expect_json()
            .query(&CurrentQueryString {
                lat: lat.to_string(),
                lon: long.to_string(),
                appid: API_KEY,
            })
            .expect("could not serialize query string")
            .build()
            .then_send(Event::SetWeather),
    }
}

#[cfg(test)]
mod tests {
    use crux_core::App as _;
    use crux_http::{
        protocol::{HttpRequest, HttpResponse, HttpResult},
        testing::ResponseBuilder,
    };

    use super::*;
    use crate::{App, Clouds, Coord, CurrentResponse, Event, Main, Model, Sys, Weather, Wind};

    #[test]
    fn test_app() {
        let app = App;
        let lat_lng = (33.456789, -112.037222);
        let event = Event::Home(HomeEvent::Show(lat_lng.0, lat_lng.1));

        let mut cmd = app.update(event, &mut Model::default(), &());

        let mut request = cmd.effects().next().unwrap().expect_http();

        // Foobar

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
        // resolve the request with a simulated response from the web API
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

        // the app should emit a `Set` event with the HTTP response
        let actual = cmd.events().next().unwrap();
        let expected = Event::SetWeather(Ok(ResponseBuilder::ok()
            .body(CurrentResponse {
                main: Main {
                    temp: 20.0,
                    feels_like: 18.0,
                    temp_min: 18.0,
                    temp_max: 22.0,
                    pressure: 1013,
                    humidity: 50,
                },
                coord: Coord {
                    lat: 33.456789,
                    lon: -112.037222,
                },
                weather: vec![Weather {
                    id: 800,
                    main: "Clear".to_string(),
                    description: "clear sky".to_string(),
                    icon: "01d".to_string(),
                }],
                base: "".to_string(),
                visibility: 10000,
                wind: Wind {
                    speed: 4.1,
                    deg: 280,
                    gust: Some(5.2),
                },
                clouds: Clouds { all: 0 },
                dt: 1716216000,
                sys: Sys {
                    id: 1,
                    country: "US".to_string(),
                    sys_type: 1,
                    sunrise: 1716216000,
                    sunset: 1716216000,
                },
                timezone: 1,
                id: 1,
                name: "Phoenix".to_string(),
                cod: 200,
            })
            .build()));
        assert_eq!(actual, expected);
    }
}
