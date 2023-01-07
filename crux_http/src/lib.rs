//! A HTTP client for use with Crux
//!
//! `crux_http` allows Crux apps to make HTTP requests by asking the Shell to perform them.
//!
//! This is still work in progress and large parts of HTTP are not yet supported.
// #![warn(missing_docs)]

use crux_core::{capability::CapabilityContext, Capability};
use http::Method;
use url::Url;

mod client;
mod config;
mod error;
mod expect;
mod middleware;
mod request;
mod request_builder;
mod response;

pub mod protocol;

// TODO: Think about this Result re-export.
pub use http_types::{self as http};

pub use self::{
    config::Config,
    error::Error,
    request::Request,
    request_builder::RequestBuilder,
    response::{Response, ResponseAsync},
};

// TODO: These are definitely temporary
pub use self::{protocol::HttpRequest, protocol::HttpResponse};

use client::Client;

pub type Result<T> = std::result::Result<T, Error>;

/// The Http capability API.
pub struct Http<Ev> {
    context: CapabilityContext<protocol::HttpRequest, Ev>,
    client: Client,
}

impl<Ev> Clone for Http<Ev> {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
            client: self.client.clone(),
        }
    }
}

impl<Ev> Http<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<protocol::HttpRequest, Ev>) -> Self {
        Self {
            client: Client::new(context.clone()),
            context,
        }
    }

    pub fn send_<F>(&self, _req: impl Into<Request>, _callback: F) {
        // Surprisingly hard to impl since I put the send func on RequestBuilder
        // and not request :(
        todo!()
    }

    // TODO: document all of these.
    pub fn get_(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::Get, url.as_ref().parse().unwrap(), self.clone())
    }
    pub fn head(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::Head, url.as_ref().parse().unwrap(), self.clone())
    }
    pub fn post_(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::Post, url.as_ref().parse().unwrap(), self.clone())
    }
    pub fn put(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::Put, url.as_ref().parse().unwrap(), self.clone())
    }
    pub fn delete(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::Delete, url.as_ref().parse().unwrap(), self.clone())
    }
    pub fn connect(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::Connect, url.as_ref().parse().unwrap(), self.clone())
    }
    pub fn options(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::Options, url.as_ref().parse().unwrap(), self.clone())
    }
    pub fn trace(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::Trace, url.as_ref().parse().unwrap(), self.clone())
    }
    pub fn patch(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::Patch, url.as_ref().parse().unwrap(), self.clone())
    }
    pub fn request(&self, method: http::Method, url: Url) -> RequestBuilder<Ev> {
        RequestBuilder::new(method, url, self.clone())
    }

    /// Instruct the Shell to perform a HTTP GET request to the provided URL
    /// When finished, a `HttpResponse` wrapped in the event returned by `callback`
    /// will be dispatched to the app's `update` function.
    pub fn get<F>(&self, url: Url, callback: F)
    where
        Ev: 'static,
        F: Fn(protocol::HttpResponse) -> Ev + Send + 'static,
    {
        self.send(protocol::HttpMethod::Get, url, callback)
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
            let request = protocol::HttpRequest {
                method: protocol::HttpMethod::Get.to_string(),
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
                method: protocol::HttpMethod::Post.to_string(),
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
    pub fn send<F>(&self, method: protocol::HttpMethod, url: Url, callback: F)
    where
        Ev: 'static,
        F: Fn(protocol::HttpResponse) -> Ev + Send + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let request = protocol::HttpRequest {
                method: method.to_string(),
                url: url.to_string(),
            };
            let resp = ctx.request_from_shell(request).await;

            ctx.update_app(callback(resp))
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
