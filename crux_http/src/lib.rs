// #![warn(missing_docs)]
//! TODO mod docs
//!
//!

use crux_core::{capability::CapabilityContext, Capability};
use url::Url;

mod client;
mod config;
mod expect;
mod middleware;
mod request;
mod request_builder;
mod response;

pub mod effect;

// TODO: Think about this Result re-export.
pub use http_types::{self as http, Error, Result};

pub use self::{
    config::Config,
    request::Request,
    request_builder::RequestBuilder,
    response::{Response, ResponseAsync},
};

// TODO: These are definitely temporary
pub use self::{effect::HttpRequest, effect::HttpResponse};

use client::Client;

#[derive(Clone)]
pub struct Http<Ev> {
    context: CapabilityContext<effect::HttpRequest, Ev>,
    client: Client,
}

impl<Ev> Http<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<effect::HttpRequest, Ev>) -> Self {
        Self {
            client: Client::new(context.clone()),
            context,
        }
    }

    pub fn send_<F>(req: impl Into<Request>, callback: F) {
        todo!()
    }

    // TODO: document all of these.
    pub fn get_(url: impl AsRef<str>) -> RequestBuilder<Ev> {
        todo!()
    }
    pub fn head(url: impl AsRef<str>) -> RequestBuilder<Ev> {
        todo!()
    }
    pub fn post(url: impl AsRef<str>) -> RequestBuilder<Ev> {
        todo!()
    }
    pub fn put(url: impl AsRef<str>) -> RequestBuilder<Ev> {
        todo!()
    }
    pub fn delete(url: impl AsRef<str>) -> RequestBuilder<Ev> {
        todo!()
    }
    pub fn connect(url: impl AsRef<str>) -> RequestBuilder<Ev> {
        todo!()
    }
    pub fn options(url: impl AsRef<str>) -> RequestBuilder<Ev> {
        todo!()
    }
    pub fn trace(url: impl AsRef<str>) -> RequestBuilder<Ev> {
        todo!()
    }
    pub fn patch(url: impl AsRef<str>) -> RequestBuilder<Ev> {
        todo!()
    }
    pub fn request(method: http::Method, url: Url) -> RequestBuilder<Ev> {
        todo!()
    }

    pub fn get<F>(&self, url: Url, callback: F)
    where
        Ev: 'static,
        F: Fn(effect::HttpResponse) -> Ev + Send + 'static,
    {
        self.send(effect::HttpMethod::Get, url, callback)
    }

    pub fn get_json<T, F>(&self, url: Url, callback: F)
    where
        T: serde::de::DeserializeOwned,
        F: Fn(T) -> Ev + Send + Clone + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let request = effect::HttpRequest {
                method: effect::HttpMethod::Get.to_string(),
                url: url.to_string(),
            };
            let resp = ctx.effect(request).await;

            let data =
                serde_json::from_slice::<T>(&resp.body).expect("TODO: do something sensible here");

            ctx.send_event(callback(data))
        });
    }

    pub fn send<F>(&self, method: effect::HttpMethod, url: Url, callback: F)
    where
        Ev: 'static,
        F: Fn(effect::HttpResponse) -> Ev + Send + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let request = effect::HttpRequest {
                method: method.to_string(),
                url: url.to_string(),
            };
            let resp = ctx.effect(request).await;

            ctx.send_event(callback(resp))
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
