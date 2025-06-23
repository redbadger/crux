use crux_core::command::RequestBuilder;
use crux_http::command::Http;
use crux_http::protocol::HttpRequest;
use serde::Serialize;

use crate::config::API_KEY;
use crate::weather::model::{CurrentResponse, WEATHER_URL};
use crate::{Effect, Event};

#[derive(Serialize)]
pub struct CurrentQueryString {
    pub lat: String,
    pub lon: String,
    pub units: &'static str,
    pub appid: String,
}

pub struct WeatherApiClient;

impl WeatherApiClient {
    /// Build an `HttpRequest` for testing purposes
    pub fn build_request(lat: f64, lon: f64) -> HttpRequest {
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
    pub fn fetch_weather(
        lat: f64,
        lon: f64,
    ) -> RequestBuilder<
        Effect,
        Event,
        impl std::future::Future<
            Output = Result<crux_http::Response<CurrentResponse>, crux_http::HttpError>,
        >,
    > {
        Http::get(WEATHER_URL)
            .expect_json()
            .query(&CurrentQueryString {
                lat: lat.to_string(),
                lon: lon.to_string(),
                units: "metric",
                appid: API_KEY.clone(),
            })
            .expect("could not serialize query string")
            .build()
    }
}
