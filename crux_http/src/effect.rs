use std::sync::Arc;

use async_trait::async_trait;
use derive_more::Display;
use serde::{Deserialize, Serialize};

// TODO: this can go in the bin.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Display)]
pub enum HttpMethod {
    #[display(fmt = "GET")]
    Get,
    #[display(fmt = "HEAD")]
    Head,
    #[display(fmt = "POST")]
    Post,
    #[display(fmt = "PUT")]
    Put,
    #[display(fmt = "DELETE")]
    Delete,
    #[display(fmt = "CONNECT")]
    Connect,
    #[display(fmt = "OPTIONS")]
    Options,
    #[display(fmt = "TRACE")]
    Trace,
    #[display(fmt = "PATCH")]
    Patch,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    // TODO support headers
}

#[derive(Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,   // FIXME this probably should be a giant enum instead.
    pub body: Vec<u8>, // TODO support headers
}

impl crux_core::Effect for HttpRequest {
    type Response = HttpResponse;
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
        crux_core::capability::CapabilityContext::effect(self, effect).await
    }
}

impl From<crate::Request> for HttpRequest {
    fn from(req: crate::Request) -> Self {
        HttpRequest {
            method: req.method().to_string(),
            url: req.url().to_string(),
        }
    }
}

impl From<HttpResponse> for crate::ResponseAsync {
    fn from(_: HttpResponse) -> Self {
        todo!()
    }
}
