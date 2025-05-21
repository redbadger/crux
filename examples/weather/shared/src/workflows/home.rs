use crux_core::Command;
use crux_http::command::Http;
use serde::{Deserialize, Serialize};

use crate::{Effect, Event, Model};

#[derive(Serialize, Deserialize, Clone)]
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

pub fn update(event: HomeEvent, model: &mut Model) -> Command<Effect, Event> {
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
