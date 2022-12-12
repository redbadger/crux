// #![warn(missing_docs)]
//! TODO mod docs
//!
//!

use crux_core::{
    capability::{CapabilityContext, Operation},
    Capability,
};
use http::Method;
use url::Url;

mod client;
mod config;
mod expect;
mod middleware;
mod request;
mod request_builder;
mod response;

pub mod protocol;

// TODO: Think about this Result re-export.
pub use http_types::{self as http, Error, Result};

pub use self::{
    config::Config,
    request::Request,
    request_builder::RequestBuilder,
    response::{Response, ResponseAsync},
};

// TODO: These are definitely temporary
pub use self::{protocol::HttpRequest, protocol::HttpResponse};

use client::Client;

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

    pub fn send_<F>(req: impl Into<Request>, callback: F) {
        todo!()
    }

    // TODO: document all of these.
    pub fn get_(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::Get, url.as_ref().parse().unwrap(), self.clone())
    }
    pub fn head(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::Head, url.as_ref().parse().unwrap(), self.clone())
    }
    pub fn post(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
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

    pub fn get<F>(&self, url: Url, callback: F)
    where
        Ev: 'static,
        F: Fn(protocol::HttpResponse) -> Ev + Send + 'static,
    {
        self.send(protocol::HttpMethod::Get, url, callback)
    }

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
