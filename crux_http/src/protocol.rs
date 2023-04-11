//! The protocol for communicating with the shell
//!
//! Crux capabilities don't interface with the outside world themselves, they carry
//! out all their operations by exchanging messages with the platform specific shell.
//! This module defines the protocol for crux_http to communicate with the shell.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<HttpHeader>,
    pub body: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,   // FIXME this probably should be a giant enum instead.
    pub body: Vec<u8>, // TODO support headers
}

impl crux_core::capability::Operation for HttpRequest {
    type Output = HttpResponse;
}

#[async_trait]
pub(crate) trait EffectSender {
    async fn send(&self, effect: HttpRequest) -> HttpResponse;
}

#[async_trait]
impl<Ev> EffectSender for crux_core::capability::CapabilityContext<HttpRequest, Ev>
where
    Ev: 'static,
{
    async fn send(&self, effect: HttpRequest) -> HttpResponse {
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
        let mut res = crate::http::Response::new(effect_response.status);
        res.set_body(effect_response.body);
        crate::ResponseAsync::new(res)
    }
}
