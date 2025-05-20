use crux_core::{
    macros::effect,
    render::{render, RenderOperation},
    Command,
};
use crux_http::{command::Http, protocol::HttpRequest};
use serde::{Deserialize, Serialize};

use crate::CurrentResponse;

// https://openweathermap.org/current
const WEATHER_URL: &str = "https://api.openweathermap.org/data/2.5/weather";
// ?lat={lat}&lon={lon}&appid={API key}
const API_KEY: &str = "42005d273a8a49c88a8173878232508";
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Event {
    Show(f64, f64),

    #[serde(skip)]
    Set(crux_http::Result<crux_http::Response<CurrentResponse>>),
}

#[effect(typegen)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct UserData {
    lat: f64,
    lon: f64,
}

// Query string example from https://openweathermap.org/current
#[derive(Serialize)]
struct CurrentQueryString {
    lat: String,
    lon: String,
    appid: &'static str,
}

#[derive(Default)]
pub struct Model {
    weather_data: CurrentResponse,
    user_data: UserData,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ViewModel {
    weather_data: CurrentResponse,
    user_data: UserData,
}

#[derive(Default)]
pub struct App;

impl crux_core::App for App {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Capabilities = (); // will be deprecated, so use unit type for now
    type Effect = Effect;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        _caps: &(), // will be deprecated, so prefix with underscore for now
    ) -> Command<Effect, Event> {
        match event {
            Event::Show(lat, long) => Http::get(WEATHER_URL)
                .expect_json()
                .query(&CurrentQueryString {
                    lat: lat.to_string(),
                    lon: long.to_string(),
                    appid: API_KEY,
                })
                .expect("could not serialize query string")
                .build()
                .then_send(Event::Set),
            Event::Set(Ok(mut response)) => {
                let data = response.take_body().unwrap();
                model.weather_data = data;
                render()
            }
            Event::Set(Err(e)) => {
                println!("{:?}", e);
                render()
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        ViewModel {
            weather_data: model.weather_data.clone(),
            user_data: model.user_data.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crux_core::App as _;
    use crux_http::{
        protocol::{HttpResponse, HttpResult},
        testing::ResponseBuilder,
    };

    use crate::{Clouds, Coord, Main, Sys, Weather, Wind};

    use super::*;

    #[test]
    fn test_app() {
        let app = App;
        let lat_lng = (33.456789, -112.037222);
        let event = Event::Show(lat_lng.0, lat_lng.1);

        let mut cmd = app.update(event, &mut Model::default(), &());

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
        let expected = Event::Set(Ok(ResponseBuilder::ok()
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
