use crux_core::Request;
use crux_core::command::RequestBuilder;
use crux_http::command::Http;
use crux_http::protocol::HttpRequest;
use serde::{Deserialize, Serialize};

use crate::effects::location::Location;
use crate::model::ApiKey;
use crate::model::active::weather::model::current_response::{CurrentWeatherResponse, WEATHER_URL};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WeatherError {
    NetworkError,
    ParseError,
}

#[derive(Serialize)]
pub struct CurrentWeatherQuery {
    pub lat: String,
    pub lon: String,
    pub units: &'static str,
    pub appid: String,
}

// ANCHOR: client
pub struct WeatherApi;

impl WeatherApi {
    /// Build an `HttpRequest` for testing purposes
    #[cfg(test)]
    pub fn build(location: Location, api_key: &ApiKey) -> HttpRequest {
        use crate::model::active::weather::model::current_response::WEATHER_URL;

        HttpRequest::get(WEATHER_URL)
            .query(&CurrentWeatherQuery {
                lat: location.lat.to_string(),
                lon: location.lon.to_string(),
                units: "metric",
                appid: api_key.clone().into(),
            })
            .expect("could not serialize query string")
            .build()
    }

    /// Fetch current weather for a specific location
    pub fn fetch<Effect, Event>(
        location: Location,
        api_key: ApiKey,
    ) -> RequestBuilder<
        Effect,
        Event,
        impl std::future::Future<Output = Result<CurrentWeatherResponse, WeatherError>>,
    >
    where
        Effect: From<Request<HttpRequest>> + Send + 'static,
        Event: Send + 'static,
    {
        Http::get(WEATHER_URL)
            .expect_json::<CurrentWeatherResponse>()
            .query(&CurrentWeatherQuery {
                lat: location.lat.to_string(),
                lon: location.lon.to_string(),
                units: "metric",
                appid: api_key.into(),
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
// ANCHOR_END: client
