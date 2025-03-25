//! The protocol for communicating with the shell
//!
//! Crux capabilities don't interface with the outside world themselves, they carry
//! out all their operations by exchanging messages with the platform specific shell.
//! This module defines the protocol for crux_http to communicate with the shell.

use async_trait::async_trait;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};

use crate::HttpError;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, Builder)]
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
        };
        builder
            .field("body", &format_args!("{}", body_repr))
            .finish()
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

    pub fn json(&mut self, body: impl serde::Serialize) -> &mut Self {
        self.body = Some(serde_json::to_vec(&body).unwrap());
        self
    }

    pub fn build(&self) -> HttpRequest {
        self.fallible_build()
            .expect("All required fields were initialized")
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq, Builder)]
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
    pub body: Vec<u8>,
}

impl HttpResponse {
    pub fn status(status: u16) -> HttpResponseBuilder {
        HttpResponseBuilder {
            status: Some(status),
            headers: Some(vec![]),
            body: Some(vec![]),
        }
    }
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

    pub fn json(&mut self, body: impl serde::Serialize) -> &mut Self {
        self.body = Some(serde_json::to_vec(&body).unwrap());
        self
    }

    pub fn build(&self) -> HttpResponse {
        self.fallible_build()
            .expect("All required fields were initialized")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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
    fn register_types(generator: &mut crux_core::typegen::TypeGen) -> crux_core::typegen::Result {
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

#[async_trait]
impl<Ev> EffectSender for crux_core::capability::CapabilityContext<HttpRequest, Ev>
where
    Ev: 'static,
{
    async fn send(&self, effect: HttpRequest) -> HttpResult {
        crux_core::capability::CapabilityContext::request_from_shell(self, effect).await
    }
}

#[async_trait]
pub(crate) trait ProtocolRequestBuilder {
    async fn into_protocol_request(mut self) -> crate::Result<HttpRequest>;
}

#[async_trait]
impl ProtocolRequestBuilder for crate::Request {
    async fn into_protocol_request(mut self) -> crate::Result<HttpRequest> {
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
}
