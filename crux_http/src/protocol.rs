//! The protocol for communicating with the shell
//!
//! Crux capabilities don't interface with the outside world themselves, they carry
//! out all their operations by exchanging messages with the platform specific shell.
//! This module defines the protocol for `crux_http` to communicate with the shell.

use crate::HttpError;
use crux_core::{BoxFuture, MaybeSend, MaybeSync};
use derive_builder::Builder;
use serde::{Deserialize, Serialize};

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
    #[facet(bytes)]
    pub body: Vec<u8>,
}

impl std::fmt::Debug for HttpRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let body_repr = if let Ok(s) = std::str::from_utf8(&self.body) {
            if s.len() < 50 {
                format!("\"{s}\"")
            } else {
                format!("\"{}\"...", s.chars().take(50).collect::<String>())
            }
        } else {
            format!("<binary data - {} bytes>", self.body.len())
        };
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
    pub fn query(&mut self, query: &impl Serialize) -> crate::Result<&mut Self> {
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
    #[facet(bytes)]
    pub body: Vec<u8>,
}

impl HttpResponse {
    #[must_use]
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

impl From<crate::Result<HttpResponse>> for HttpResult {
    fn from(result: Result<HttpResponse, HttpError>) -> Self {
        match result {
            Ok(response) => HttpResult::Ok(response),
            Err(err) => HttpResult::Err(err),
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

pub(crate) trait EffectSender: MaybeSend + MaybeSync {
    fn send(&self, effect: HttpRequest) -> BoxFuture<'_, HttpResult>;
}

#[expect(deprecated)]
impl<Ev> EffectSender for crux_core::capability::CapabilityContext<HttpRequest, Ev>
where
    Ev: 'static,
{
    fn send(&self, effect: HttpRequest) -> BoxFuture<'_, HttpResult> {
        Box::pin(crux_core::capability::CapabilityContext::request_from_shell(self, effect))
    }
}

pub(crate) trait ProtocolRequestBuilder {
    fn into_protocol_request(self) -> BoxFuture<'static, crate::Result<HttpRequest>>;
}

impl ProtocolRequestBuilder for crate::Request {
    fn into_protocol_request(mut self) -> BoxFuture<'static, crate::Result<HttpRequest>> {
        Box::pin(async move {
            let body = if self.is_empty() == Some(false) {
                self.take_body().into_bytes().await?
            } else {
                vec![]
            };

            Ok(HttpRequest {
                method: self.method().to_string(),
                url: self.url().to_string(),
                headers: self
                    .iter()
                    .flat_map(|(name, values)| {
                        values.iter().map(|value| HttpHeader {
                            name: name.to_string(),
                            value: value.to_string(),
                        })
                    })
                    .collect(),
                body,
            })
        })
    }
}

impl From<HttpResponse> for crate::ResponseAsync {
    fn from(effect_response: HttpResponse) -> Self {
        let mut res = http_types::Response::new(effect_response.status);
        res.set_body(effect_response.body);
        for header in effect_response.headers {
            res.append_header(header.name.as_str(), header.value);
        }

        crate::ResponseAsync::new(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[test]
    fn test_http_request_get() {
        let req = HttpRequest::get("https://example.com").build();

        assert_eq!(
            req,
            HttpRequest {
                method: "GET".to_string(),
                url: "https://example.com".to_string(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_http_request_get_with_fields() {
        let req = HttpRequest::get("https://example.com")
            .header("foo", "bar")
            .body("123")
            .build();

        assert_eq!(
            req,
            HttpRequest {
                method: "GET".to_string(),
                url: "https://example.com".to_string(),
                headers: vec![HttpHeader {
                    name: "foo".to_string(),
                    value: "bar".to_string(),
                }],
                body: "123".as_bytes().to_vec(),
            }
        );
    }

    #[test]
    fn test_http_response_status() {
        let req = HttpResponse::status(302).build();

        assert_eq!(
            req,
            HttpResponse {
                status: 302,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_http_response_status_with_fields() {
        let req = HttpResponse::status(302)
            .header("foo", "bar")
            .body("hello world")
            .build();

        assert_eq!(
            req,
            HttpResponse {
                status: 302,
                headers: vec![HttpHeader {
                    name: "foo".to_string(),
                    value: "bar".to_string(),
                }],
                body: "hello world".as_bytes().to_vec(),
            }
        );
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

        assert_eq!(
            req,
            HttpRequest {
                method: "GET".to_string(),
                url: "https://example.com?page=2&limit=10&search=test".to_string(),
                headers: vec![HttpHeader {
                    name: "foo".to_string(),
                    value: "bar".to_string(),
                }],
                body: vec![],
            }
        );
    }

    #[test]
    fn test_http_request_query_with_special_chars() {
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
            url: Some("https://example.com".to_string()),
            headers: Some(vec![]),
            body: Some(vec![]),
        };

        builder
            .query(&query)
            .expect("should serialize query params with special chars");
        let req = builder.build();

        assert_eq!(
            req,
            HttpRequest {
                method: "GET".to_string(),
                url: "https://example.com?name=John+Doe&email=john%40example.com".to_string(),
                headers: vec![],
                body: vec![],
            }
        );
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

        assert_eq!(
            req,
            HttpRequest {
                method: "GET".to_string(),
                url: "https://example.com?empty=".to_string(),
                headers: vec![],
                body: vec![],
            }
        );
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

        assert_eq!(
            req,
            HttpRequest {
                method: "GET".to_string(),
                url: "https://example.com?foo=bar&name=John+Doe&email=john%40example.com"
                    .to_string(),
                headers: vec![],
                body: vec![],
            }
        );
    }
}
