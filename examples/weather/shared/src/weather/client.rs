use crux_core::command::RequestBuilder;
use crux_http::command::Http;
use crux_http::protocol::HttpRequest;
use serde::{Deserialize, Serialize};

use crate::config::API_KEY;
use crate::weather::model::{CurrentResponse, WEATHER_URL};
use crate::{Effect, Event};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WeatherError {
    NetworkError,
    ParseError,
}

#[derive(Serialize)]
pub struct CurrentQueryString {
    pub lat: String,
    pub lon: String,
    pub units: &'static str,
    pub appid: String,
}

pub struct WeatherApi;

impl WeatherApi {
    /// Build an `HttpRequest` for testing purposes
    pub fn build(lat: f64, lon: f64) -> HttpRequest {
        HttpRequest::get(WEATHER_URL)
            .query(&CurrentQueryString {
                lat: lat.to_string(),
                lon: lon.to_string(),
                units: "metric",
                appid: API_KEY.clone(),
            })
            .expect("could not serialize query string")
            .build()
    }

    /// Fetch current weather for a specific location
    pub fn fetch(
        lat: f64,
        lon: f64,
    ) -> RequestBuilder<
        Effect,
        Event,
        impl std::future::Future<Output = Result<CurrentResponse, WeatherError>>,
    > {
        Http::get(WEATHER_URL)
            .expect_json::<CurrentResponse>()
            .query(&CurrentQueryString {
                lat: lat.to_string(),
                lon: lon.to_string(),
                units: "metric",
                appid: API_KEY.clone(),
            })
            .expect("could not serialize query string")
            .build()
            .map(|result| match result {
                Ok(mut response) => match response.take_body() {
                    Some(weather_data) => Ok(weather_data),
                    None => Err(WeatherError::ParseError),
                },
                Err(_) => Err(WeatherError::NetworkError),
            })
    }
}
