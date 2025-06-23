use crux_core::command::RequestBuilder;
use crux_http::command::Http;
use crux_http::protocol::HttpRequest;
use serde::Serialize;

use crate::config::API_KEY;
use crate::{Effect, GeocodingQueryString, GeocodingResponse, GEOCODING_URL};

#[derive(Serialize)]
pub struct CurrentQueryString {
    pub lat: String,
    pub lon: String,
    pub units: &'static str,
    pub appid: String,
}

pub struct LocationApi;

impl LocationApi {
    /// Build an `HttpRequest` for testing purposes
    pub fn build(query: &str) -> HttpRequest {
        HttpRequest::get(GEOCODING_URL)
            .query(&GeocodingQueryString {
                q: query.to_string(),
                limit: "5",
                appid: API_KEY.clone(),
            })
            .expect("could not serialize query string")
            .build()
    }

    /// Fetch current weather for a specific location
    pub fn fetch<Event>(
        query: &str,
    ) -> RequestBuilder<
        Effect,
        Event,
        impl std::future::Future<
            Output = Result<crux_http::Response<Vec<GeocodingResponse>>, crux_http::HttpError>,
        >,
    >
    where
        Event: Send + 'static,
    {
        Http::get(GEOCODING_URL)
            .expect_json()
            .query(&GeocodingQueryString {
                q: query.to_string(),
                limit: "5",
                appid: API_KEY.clone(),
            })
            .expect("could not serialize query string")
            .build()
    }
}
