//! TODO mod docs

use crux_core::{Capability, Command};
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

pub struct Http<Ef> {
    make_effect: Box<dyn Fn(HttpRequest) -> Ef + Sync>,
}

impl<Ef> Http<Ef> {
    pub fn new<MakeEffect>(make_effect: MakeEffect) -> Self
    where
        MakeEffect: Fn(HttpRequest) -> Ef + Sync + 'static,
    {
        Self {
            make_effect: Box::new(make_effect),
        }
    }

    pub fn get<Ev, F>(&self, url: Url, callback: F) -> Command<Ef, Ev>
    where
        Ev: 'static,
        F: Fn(HttpResponse) -> Ev + Send + Sync + 'static,
    {
        self.request(HttpMethod::Get, url, callback)
    }

    pub fn request<Ev, F>(&self, method: HttpMethod, url: Url, callback: F) -> Command<Ef, Ev>
    where
        Ev: 'static,
        F: Fn(HttpResponse) -> Ev + Send + Sync + 'static,
    {
        let request = HttpRequest {
            method: method.to_string(),
            url: url.to_string(),
        };

        Command::new((self.make_effect)(request), callback)
    }
}

impl<Ef> Capability for Http<Ef> where Ef: Clone {}
