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
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,   // FIXME this probably should be a giant enum instead.
    pub body: Vec<u8>, // TODO support headers
}

impl Operation for HttpRequest {
    type Output = HttpResponse;
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
        F: Fn(HttpResponse) -> Ev + Send + 'static,
    {
        self.send(HttpMethod::Get, url, callback)
    }

    /// Instruct the Shell to perform a HTTP GET request to the provided `url`, expecting
    /// a JSON response.
    ///
    /// When finished, the response will be deserialized into type `T`, wrapped
    /// in an event using `callback` and dispatched to the app's `update function.
    pub fn get_json<T, F>(&self, url: Url, callback: F)
    where
        T: serde::de::DeserializeOwned,
        F: Fn(T) -> Ev + Send + Clone + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let request = HttpRequest {
                method: HttpMethod::Get.to_string(),
                url: url.to_string(),
            };
            let resp = ctx.request_from_shell(request).await;

            let data =
                serde_json::from_slice::<T>(&resp.body).expect("TODO: do something sensible here");

            ctx.update_app(callback(data))
        });
    }

    /// Instruct the Shell to perform a HTTP POST request to the provided `url`, expecting
    /// a JSON response.
    ///
    /// When finished, the response will be deserialized into type `T`, wrapped
    /// in an event using `callback` and dispatched to the app's `update function.
    pub fn post<Res, F>(&self, url: Url, callback: F)
    where
        Res: serde::de::DeserializeOwned,
        F: Fn(Res) -> Ev + Send + Clone + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let request = HttpRequest {
                method: HttpMethod::Post.to_string(),
                url: url.to_string(),
            };
            let resp = ctx.request_from_shell(request).await;

            let data = serde_json::from_slice::<Res>(&resp.body)
                .expect("TODO: do something sensible here");

            ctx.update_app(callback(data))
        });
    }

    /// Instruct the Shell to perform a HTTP request with the provided `method` to the provided `url`.
    ///
    /// When finished, a `HttpResponse` wrapped in the event returned by `callback`
    /// will be dispatched to the app's `update` function.
    pub fn send<F>(&self, method: HttpMethod, url: Url, callback: F)
    where
        Ev: 'static,
        F: Fn(HttpResponse) -> Ev + Send + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let request = HttpRequest {
                method: method.to_string(),
                url: url.to_string(),
            };
            let resp = ctx.request_from_shell(request).await;

            ctx.update_app(callback(resp))
        });
    }
}

impl<Ef> Capability<Ef> for Http<Ef> {
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
