//! A HTTP client for use with Crux
//!
//! `crux_http` allows Crux apps to make HTTP requests by asking the Shell to perform them.
//!
//! This is still work in progress and large parts of HTTP are not yet supported.

use crux_core::{
    capability::{CapabilityContext, Operation},
    Capability,
};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

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

/// A HTTP request
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    // TODO support headers
}

/// A HTTP Response with body stored as bytes
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct HttpResponse {
    pub status: u16,           // FIXME this probably should be a giant enum instead.
    pub body: Option<Vec<u8>>, // TODO support headers
}

impl Operation for HttpRequest {
    type Output = Result<HttpResponse, HttpError>;
}

#[derive(Error, Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[error("method: {method}, url: {url}, error: {error}")]
pub struct HttpError {
    pub method: String,
    pub url: String,
    pub error: String,
}

/// The Http capability API.
pub struct Http<Ev> {
    context: CapabilityContext<HttpRequest, Ev>,
}

impl<Ev> Http<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<HttpRequest, Ev>) -> Self {
        Self { context }
    }

    /// Instruct the Shell to perform a HTTP GET request to the provided URL
    /// When finished, a `HttpResponse` wrapped in the event returned by `callback`
    /// will be dispatched to the app's `update` function.
    pub fn get<F>(&self, url: Url, callback: F)
    where
        Ev: 'static,
        F: Fn(Result<HttpResponse, HttpError>) -> Ev + Send + 'static,
    {
        self.send(HttpMethod::Get, url, callback)
    }

    /// Instruct the Shell to perform a HTTP POST request to the provided URL
    /// When finished, a `HttpResponse` wrapped in the event returned by `callback`
    /// will be dispatched to the app's `update` function.
    pub fn post<F>(&self, url: Url, callback: F)
    where
        Ev: 'static,
        F: Fn(Result<HttpResponse, HttpError>) -> Ev + Send + 'static,
    {
        self.send(HttpMethod::Post, url, callback)
    }

    /// Instruct the Shell to perform a HTTP request with the provided `method` to the provided `url`.
    ///
    /// When finished, a `HttpResponse` wrapped in the event returned by `callback`
    /// will be dispatched to the app's `update` function.
    pub fn send<F>(&self, method: HttpMethod, url: Url, callback: F)
    where
        Ev: 'static,
        F: Fn(Result<HttpResponse, HttpError>) -> Ev + Send + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let request = HttpRequest {
                method: method.to_string(),
                url: url.to_string(),
            };
            match ctx.request_from_shell(request).await {
                Ok(resp) => ctx.update_app(callback(Ok(resp))),
                Err(e) => ctx.update_app(callback(Err(e))),
            }
        });
    }
}

impl<Ef> Capability<Ef> for Http<Ef> {
    type Operation = HttpRequest;
    type MappedSelf<MappedEv> = Http<MappedEv>;

    fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
    where
        F: Fn(NewEvent) -> Ef + Send + Sync + Copy + 'static,
        Ef: 'static,
        NewEvent: 'static,
    {
        Http::new(self.context.map_event(f))
    }
}
