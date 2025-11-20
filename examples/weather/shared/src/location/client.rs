use crux_core::{Request, command::RequestBuilder};
use crux_http::command::Http;
use crux_http::protocol::HttpRequest;
use serde::{Deserialize, Serialize};

use crate::{
    config::API_KEY,
    location::model::{GEOCODING_URL, GeocodingQueryString, GeocodingResponse},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LocationError {
    NetworkError,
    ParseError,
    NoResults,
}

#[derive(Serialize)]
#[allow(dead_code)] // TODO: why?
pub struct CurrentQueryString {
    pub lat: String,
    pub lon: String,
    pub units: &'static str,
    pub appid: String,
}

pub struct LocationApi;

impl LocationApi {
    /// Build an `HttpRequest` for testing purposes
    #[cfg(test)]
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

    /// Fetch geocoding results for a location query
    pub fn fetch<Event, Effect>(
        query: &str,
    ) -> RequestBuilder<
        Effect,
        Event,
        impl std::future::Future<Output = Result<Vec<GeocodingResponse>, LocationError>>
        + use<Event, Effect>,
    >
    where
        Event: Send + 'static,
        Effect: From<Request<HttpRequest>> + Send + 'static,
    {
        Http::get(GEOCODING_URL)
            .expect_json::<Vec<GeocodingResponse>>()
            .query(&GeocodingQueryString {
                q: query.to_string(),
                limit: "5",
                appid: API_KEY.clone(),
            })
            .expect("could not serialize query string")
            .build()
            .map(|result| match result {
                Ok(mut response) => match response.take_body() {
                    Some(results) => {
                        if results.is_empty() {
                            Err(LocationError::NoResults)
                        } else {
                            Ok(results)
                        }
                    }
                    None => Err(LocationError::ParseError),
                },
                Err(_) => Err(LocationError::NetworkError),
            })
    }
}
