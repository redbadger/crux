//! The protocol for communicating with the shell
//!
//! Crux capabilities don't interface with the outside world themselves, they carry
//! out all their operations by exchanging messages with the platform specific shell.
//! This module defines the protocol for `crux_http` to communicate with the shell.

use async_trait::async_trait;
use derive_builder::Builder;
use facet_generate_attrs as typegen;
use serde::{Deserialize, Serialize};

use crate::{HttpError, Request, Result};

#[derive(facet::Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

#[derive(facet::Facet, Serialize, Deserialize, Default, Clone, PartialEq, Eq, Builder)]
#[builder(
    custom_constructor,
    build_fn(private, name = "fallible_build"),
    setter(into)
)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    #[builder(setter(custom))]
    pub headers: Vec<HttpHeader>,
    #[serde(with = "serde_bytes")]
    #[facet(typegen::bytes)]
    pub body: Vec<u8>,
}

impl std::fmt::Debug for HttpRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let body_repr = std::str::from_utf8(&self.body).map_or_else(
            |_| format!("<binary data - {} bytes>", self.body.len()),
            |s| {
                if s.len() < 50 {
                    format!("\"{s}\"")
                } else {
                    format!("\"{}\"...", s.chars().take(50).collect::<String>())
                }
            },
        );
        let mut builder = f.debug_struct("HttpRequest");
        builder
            .field("method", &self.method)
            .field("url", &self.url);
        if !self.headers.is_empty() {
            builder.field("headers", &self.headers);
        }
        builder.field("body", &format_args!("{body_repr}")).finish()
    }
}

macro_rules! http_method {
    ($name:ident, $method:expr) => {
        pub fn $name(url: impl Into<String>) -> HttpRequestBuilder {
            HttpRequestBuilder {
                method: Some($method.to_string()),
                url: Some(url.into()),
                headers: Some(vec![]),
                body: Some(vec![]),
            }
        }
    };
}

impl HttpRequest {
    http_method!(get, "GET");
    http_method!(put, "PUT");
    http_method!(delete, "DELETE");
    http_method!(post, "POST");
    http_method!(patch, "PATCH");
    http_method!(head, "HEAD");
    http_method!(options, "OPTIONS");
}

impl HttpRequestBuilder {
    pub fn header(&mut self, name: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.headers.get_or_insert_with(Vec::new).push(HttpHeader {
            name: name.into(),
            value: value.into(),
        });
        self
    }

    /// Sets the query parameters of the request to the given value.
    ///
    /// # Errors
    /// Returns an [`HttpError`] if the serialization fails.
    pub fn query(&mut self, query: &impl Serialize) -> Result<&mut Self> {
        if let Some(url) = &mut self.url {
            if url.contains('?') {
                url.push('&');
            } else {
                url.push('?');
            }
            url.push_str(&serde_qs::to_string(query)?);
        }

        Ok(self)
    }

    /// Sets the body of the request to the JSON representation of the given value.
    ///
    /// # Panics
    /// Panics if the serialization fails.
    pub fn json(&mut self, body: impl serde::Serialize) -> &mut Self {
        self.body = Some(serde_json::to_vec(&body).unwrap());
        self
    }

    /// Builds the request.
    ///
    /// # Panics
    /// Panics if any required fields are missing.
    #[must_use]
    pub fn build(&self) -> HttpRequest {
        self.fallible_build()
            .expect("All required fields were initialized")
    }
}

#[derive(facet::Facet, Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq, Builder)]
#[builder(
    custom_constructor,
    build_fn(private, name = "fallible_build"),
    setter(into)
)]
pub struct HttpResponse {
    pub status: u16, // FIXME this probably should be a giant enum instead.
    #[builder(setter(custom))]
    pub headers: Vec<HttpHeader>,
    #[serde(with = "serde_bytes")]
    #[facet(typegen::bytes)]
    pub body: Vec<u8>,
}

impl HttpResponse {
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn status(status: u16) -> HttpResponseBuilder {
        HttpResponseBuilder {
            status: Some(status),
            headers: Some(vec![]),
            body: Some(vec![]),
        }
    }
    #[must_use]
    pub fn ok() -> HttpResponseBuilder {
        Self::status(200)
    }
}

impl HttpResponseBuilder {
    pub fn header(&mut self, name: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.headers.get_or_insert_with(Vec::new).push(HttpHeader {
            name: name.into(),
            value: value.into(),
        });
        self
    }

    /// Sets the body of the response to the given JSON.
    ///
    /// # Panics
    /// If the JSON serialization fails.
    pub fn json(&mut self, body: impl serde::Serialize) -> &mut Self {
        self.body = Some(serde_json::to_vec(&body).unwrap());
        self
    }

    /// Builds the response.
    ///
    /// # Panics
    /// If a required field has not been initialized.
    #[must_use]
    pub fn build(&self) -> HttpResponse {
        self.fallible_build()
            .expect("All required fields were initialized")
    }
}

#[derive(facet::Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum HttpResult {
    Ok(HttpResponse),
    Err(HttpError),
}

impl From<Result<HttpResponse>> for HttpResult {
    fn from(result: Result<HttpResponse>) -> Self {
        match result {
            Ok(response) => Self::Ok(response),
            Err(err) => Self::Err(err),
        }
    }
}

impl crux_core::capability::Operation for HttpRequest {
    type Output = HttpResult;

    #[cfg(feature = "typegen")]
    fn register_types(
        generator: &mut crux_core::type_generation::serde::TypeGen,
    ) -> crux_core::type_generation::serde::Result {
        generator.register_type::<HttpError>()?;
        generator.register_type::<Self>()?;
        generator.register_type::<Self::Output>()?;
        Ok(())
    }
}

#[async_trait]
pub(crate) trait EffectSender {
    async fn send(&self, effect: HttpRequest) -> HttpResult;
}

pub(crate) trait ProtocolRequestBuilder {
    fn into_protocol_request(self) -> Result<HttpRequest>;
}

impl ProtocolRequestBuilder for Request {
    fn into_protocol_request(mut self) -> Result<HttpRequest> {
        let body = self.take_body().into_bytes();

        Ok(HttpRequest {
            method: self.method().to_string(),
            url: self.url().to_string(),
            headers: self
                .iter()
                .map(|(name, value)| HttpHeader {
                    name: name.to_string(),
                    value: value.to_str().unwrap_or("").to_string(),
                })
                .collect(),
            body,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[test]
    fn test_http_request_get() {
        let req = HttpRequest::get("https://example.com").build();

        insta::assert_debug_snapshot!(req, @r#"
        HttpRequest {
            method: "GET",
            url: "https://example.com",
            body: "",
        }
        "#);
    }

    #[test]
    fn test_http_request_get_with_fields() {
        let req = HttpRequest::get("https://example.com")
            .header("foo", "bar")
            .body("123")
            .build();

        insta::assert_debug_snapshot!(req, @r#"
        HttpRequest {
            method: "GET",
            url: "https://example.com",
            headers: [
                HttpHeader {
                    name: "foo",
                    value: "bar",
                },
            ],
            body: "123",
        }
        "#);
    }

    #[test]
    fn test_http_response_status() {
        let req = HttpResponse::status(302).build();

        insta::assert_debug_snapshot!(req, @"
        HttpResponse {
            status: 302,
            headers: [],
            body: [],
        }
        ");
    }

    #[test]
    fn test_http_response_status_with_fields() {
        let req = HttpResponse::status(302)
            .header("foo", "bar")
            .body("hey")
            .build();

        insta::assert_debug_snapshot!(req, @r#"
        HttpResponse {
            status: 302,
            headers: [
                HttpHeader {
                    name: "foo",
                    value: "bar",
                },
            ],
            body: [
                104,
                101,
                121,
            ],
        }
        "#);
    }

    #[test]
    fn test_http_request_debug_repr() {
        {
            // small
            let req = HttpRequest::post("http://example.com")
                .header("foo", "bar")
                .body("hello world!")
                .build();
            let repr = format!("{req:?}");
            assert_eq!(
                repr,
                r#"HttpRequest { method: "POST", url: "http://example.com", headers: [HttpHeader { name: "foo", value: "bar" }], body: "hello world!" }"#
            );
        }

        {
            // big
            let req = HttpRequest::post("http://example.com")
                // we check that we handle unicode boundaries correctly
                .body("abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstu😀😀😀😀😀😀")
                .build();
            let repr = format!("{req:?}");
            assert_eq!(
                repr,
                r#"HttpRequest { method: "POST", url: "http://example.com", body: "abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstu😀😀"... }"#
            );
        }

        {
            // binary
            let req = HttpRequest::post("http://example.com")
                .body(vec![255, 254, 253, 252])
                .build();
            let repr = format!("{req:?}");
            assert_eq!(
                repr,
                r#"HttpRequest { method: "POST", url: "http://example.com", body: <binary data - 4 bytes> }"#
            );
        }
    }

    #[test]
    fn test_http_request_query() {
        #[derive(Serialize, Deserialize)]
        struct QueryParams {
            page: u32,
            limit: u32,
            search: String,
        }

        let query = QueryParams {
            page: 2,
            limit: 10,
            search: "test".to_string(),
        };

        let mut builder = HttpRequestBuilder {
            method: Some("GET".to_string()),
            url: Some("https://example.com".to_string()),
            headers: Some(vec![HttpHeader {
                name: "foo".to_string(),
                value: "bar".to_string(),
            }]),
            body: Some(vec![]),
        };

        builder
            .query(&query)
            .expect("should serialize query params");
        let req = builder.build();

        insta::assert_debug_snapshot!(req, @r#"
        HttpRequest {
            method: "GET",
            url: "https://example.com?page=2&limit=10&search=test",
            headers: [
                HttpHeader {
                    name: "foo",
                    value: "bar",
                },
            ],
            body: "",
        }
        "#);
    }

    #[test]
    fn test_http_request_query_with_special_chars() {
        #[derive(Serialize, Deserialize)]
        struct QueryParams {
            allowed: String,
            disallowed: String,
            delimiters: String,
            alpha_numeric_and_space: String,
        }

        let query = QueryParams {
            // allowed chars (RFC 3986)
            allowed: ";/?:@$,-.!~*'()".to_string(),
            // disallowed chars (RFC 3986)
            disallowed: "#".to_string(),
            // delimiters in key value pairs, need encoding
            delimiters: "&=+".to_string(),
            // not RFC 3986 Compliant (space should be %20 not +)
            // but "+" is very common so we allow it
            alpha_numeric_and_space: "ABC abc 123".to_string(),
        };

        let mut builder = HttpRequestBuilder {
            method: Some("GET".to_string()),
            url: Some("https://example.com".to_string()),
            headers: Some(vec![]),
            body: Some(vec![]),
        };

        builder
            .query(&query)
            .expect("should serialize query params with special chars");
        let req = builder.build();

        insta::assert_debug_snapshot!(req, @r#"
        HttpRequest {
            method: "GET",
            url: "https://example.com?allowed=;/?:@$,-.!~*'()&disallowed=%23&delimiters=%26%3D%2B&alpha_numeric_and_space=ABC+abc+123",
            body: "",
        }
        "#);
    }

    #[test]
    fn test_http_request_query_with_empty_values() {
        #[derive(Serialize, Deserialize)]
        struct QueryParams {
            empty: String,
            none: Option<String>,
        }

        let query = QueryParams {
            empty: String::new(),
            none: None,
        };

        let mut builder = HttpRequestBuilder {
            method: Some("GET".to_string()),
            url: Some("https://example.com".to_string()),
            headers: Some(vec![]),
            body: Some(vec![]),
        };

        builder
            .query(&query)
            .expect("should serialize query params with empty values");
        let req = builder.build();

        insta::assert_debug_snapshot!(req, @r#"
        HttpRequest {
            method: "GET",
            url: "https://example.com?empty=&none",
            body: "",
        }
        "#);
    }

    #[test]
    fn test_http_request_query_with_url_with_existing_query_params() {
        #[derive(Serialize, Deserialize)]
        struct QueryParams {
            name: String,
            email: String,
        }

        let query = QueryParams {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
        };

        let mut builder = HttpRequestBuilder {
            method: Some("GET".to_string()),
            url: Some("https://example.com?foo=bar".to_string()),
            headers: Some(vec![]),
            body: Some(vec![]),
        };

        builder
            .query(&query)
            .expect("should serialize query params");
        let req = builder.build();

        insta::assert_debug_snapshot!(req, @r#"
        HttpRequest {
            method: "GET",
            url: "https://example.com?foo=bar&name=John+Doe&email=john@example.com",
            body: "",
        }
        "#);
    }

    #[test]
    fn into_protocol_request_is_synchronous_and_carries_body() {
        use crate::{Request, Url, protocol::ProtocolRequestBuilder};
        use http::Method;

        let mut req = Request::new(Method::POST, Url::parse("https://example.com").unwrap());
        req.body_json(&serde_json::json!({"x": 1})).unwrap();

        // into_protocol_request is now a plain (sync) fn — no .await needed.
        let http_req = req.into_protocol_request().expect("must not fail");

        assert_eq!(http_req.method, "POST");
        assert_eq!(http_req.url, "https://example.com/");
        assert!(!http_req.body.is_empty(), "body must be present");

        // Content-Type header must be present in the serialised headers.
        let has_content_type = http_req.headers.iter().any(|h| {
            h.name.to_lowercase() == "content-type" && h.value.contains("application/json")
        });
        assert!(
            has_content_type,
            "Content-Type: application/json header expected"
        );
    }

    /// Round-trip: `http::Request<Body>` → `crux_http::Request` → `HttpRequest`
    #[test]
    fn http_request_body_round_trip_to_protocol() {
        use crate::{Body, Request, protocol::ProtocolRequestBuilder};

        let http_req = http::Request::builder()
            .method(http::Method::POST)
            .uri("https://api.example.com/items")
            .header("content-type", "application/json")
            .body(Body::from_json(&serde_json::json!({"name": "widget"})).unwrap())
            .unwrap();

        let req: Request = http_req.into();
        let protocol_req = req.into_protocol_request().expect("should convert");

        assert_eq!(protocol_req.method, "POST");
        assert_eq!(protocol_req.url, "https://api.example.com/items");
        assert!(!protocol_req.body.is_empty(), "body bytes must be present");
        assert!(
            protocol_req.headers.iter().any(|h| {
                h.name.to_lowercase() == "content-type" && h.value.contains("application/json")
            }),
            "Content-Type: application/json must be in headers"
        );
        // Deserialise the body back to confirm bytes are correct.
        let parsed: serde_json::Value = serde_json::from_slice(&protocol_req.body).unwrap();
        assert_eq!(parsed["name"], "widget");
    }
}
