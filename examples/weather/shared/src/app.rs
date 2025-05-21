use crux_core::{
    macros::effect,
    render::{render, RenderOperation},
    Command,
};
use crux_http::{command::Http, protocol::HttpRequest};
use serde::{Deserialize, Serialize};

use crate::{CurrentResponse, GeocodingResponse};

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

// Query string example from https://openweathermap.org/current
#[derive(Serialize)]
struct CurrentQueryString {
    lat: String,
    lon: String,
    appid: &'static str,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
enum Workflow {
    Home,
    Favorites,
    AddFavorite,
    ConfirmDeleteFavorite(u32),
}

impl Default for Workflow {
    fn default() -> Self {
        Workflow::Home
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct Favorite {
    geo: GeocodingResponse,
    current: Option<CurrentResponse>,
}

impl From<GeocodingResponse> for Favorite {
    fn from(geo: GeocodingResponse) -> Self {
        Favorite { geo, current: None }
    }
}

#[derive(Default)]
pub struct Model {
    weather_data: CurrentResponse,
    page: Workflow,
    favorites: Vec<Favorite>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ViewModel {
    pub workflow: WorkflowViewModel,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum WorkflowViewModel {
    Home { weather_data: CurrentResponse },
    Favorites { favorites: Vec<FavoriteView> },
    AddFavorite,
    ConfirmDeleteFavorite { index: u32, name: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FavoriteView {
    name: String,
    lat: f64,
    lon: f64,
    summary: Option<String>,
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

    fn view(&self, model: &Model) -> ViewModel {
        let workflow = match &model.page {
            Workflow::Home => WorkflowViewModel::Home {
                weather_data: model.weather_data.clone(),
            },
            Workflow::Favorites => WorkflowViewModel::Favorites {
                favorites: model
                    .favorites
                    .iter()
                    .map(|f| FavoriteView {
                        name: f.geo.name.clone(),
                        lat: f.geo.lat,
                        lon: f.geo.lon,
                        summary: f.current.as_ref().map(|c| {
                            format!(
                                "{}°C, {}",
                                c.main.temp,
                                c.weather
                                    .get(0)
                                    .map(|w| w.description.clone())
                                    .unwrap_or_default()
                            )
                        }),
                    })
                    .collect(),
            },
            Workflow::AddFavorite => WorkflowViewModel::AddFavorite,
            Workflow::ConfirmDeleteFavorite(index) => WorkflowViewModel::ConfirmDeleteFavorite {
                index: *index,
                name: model
                    .favorites
                    .get(*index as usize)
                    .map(|f| f.geo.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string()),
            },
        };

        ViewModel { workflow }
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
