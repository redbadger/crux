use crux_core::{
    macros::effect,
    render::{render, RenderOperation},
    Command,
};
use crux_http::protocol::HttpRequest;
use serde::{Deserialize, Serialize};

use crate::{
    workflows::{
        self,
        favorites::{Favorite, FavoritesState},
        AddFavoriteEvent, FavoritesEvent, HomeEvent,
    },
    CurrentResponse,
};

// https://openweathermap.org/current
const WEATHER_URL: &str = "https://api.openweathermap.org/data/2.5/weather";
// ?lat={lat}&lon={lon}&appid={API key}
const API_KEY: &str = "42005d273a8a49c88a8173878232508";

#[derive(Serialize, Deserialize, Clone)]
pub enum Event {
    Navigate(Workflow),
    Home(HomeEvent),
    Favorites(FavoritesEvent),
    AddFavorite(AddFavoriteEvent),

    // Internal events
    #[serde(skip)]
    SetWeather(crux_http::Result<crux_http::Response<CurrentResponse>>),
}

#[effect(typegen)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Workflow {
    Home,
    Favorites(FavoritesState),
    AddFavorite,
}

impl Default for Workflow {
    fn default() -> Self {
        Workflow::Home
    }
}

#[derive(Default)]
pub struct Model {
    pub weather_data: CurrentResponse,
    pub page: Workflow,
    pub favorites: Vec<Favorite>,
    pub show_add_modal: bool,
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
    ConfirmDeleteFavorite { lat: f64, lng: f64 },
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
        _caps: &(),
    ) -> Command<Effect, Event> {
        match event {
            Event::Navigate(page) => {
                model.page = page;
                render()
            }
            Event::Home(home_event) => workflows::update_home(home_event, model),
            Event::Favorites(fav_event) => workflows::update_favorites(fav_event, model),
            Event::AddFavorite(add_event) => workflows::update_add_favorite(add_event, model),

            Event::SetWeather(Ok(mut response)) => {
                let data = response.take_body().unwrap();
                model.weather_data = data;
                render()
            }
            Event::SetWeather(Err(e)) => {
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
            Workflow::Favorites(FavoritesState::Idle) => WorkflowViewModel::Favorites {
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
            Workflow::Favorites(FavoritesState::ConfirmDelete(lat, lng)) => {
                WorkflowViewModel::ConfirmDeleteFavorite {
                    lat: *lat,
                    lng: *lng,
                }
            }
            Workflow::AddFavorite => WorkflowViewModel::AddFavorite,
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

    use crate::{Clouds, Coord, GeocodingResponse, Main, Sys, Weather, Wind};

    use super::*;

    #[test]
    fn test_app() {
        let app = App;
        let lat_lng = (33.456789, -112.037222);
        let event = Event::Favorites(FavoritesEvent::DeletePressed(Favorite {
            geo: GeocodingResponse {
                lat: lat_lng.0,
                lon: lat_lng.1,
                name: "Phoenix".to_string(),
                local_names: None,
                country: "US".to_string(),
                state: None,
            },
            current: None,
        }));

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
