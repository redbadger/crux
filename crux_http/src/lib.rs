//! TODO mod docs

use std::{
    marker::PhantomData,
    sync::{mpsc::Sender, Arc, Mutex},
};

use bcs::from_bytes;
use crux_core::{command::Callback, Command};
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

pub struct Http<Ev> {
    // TODO: On wasm this'll need to be an Rc<RefCell<VecDeque<T>>> or w/e - build a wrapper.
    // Or at least check if we need to.  Probably also incorporate the mutex into that wrapper for ease of use...
    sender: crux_core::channels::Sender<Command<HttpRequest, Ev>>,
}

impl<Ev> Http<Ev>
where
    Ev: 'static,
{
    pub fn new(sender: crux_core::channels::Sender<Command<HttpRequest, Ev>>) -> Self {
        Self { sender }
    }

    pub fn get<F>(&self, url: Url, callback: F)
    where
        Ev: 'static,
        F: Fn(HttpResponse) -> Ev + Send + 'static,
    {
        self.send(HttpMethod::Get, url, callback)
    }

    pub fn send<F>(&self, method: HttpMethod, url: Url, callback: F)
    where
        Ev: 'static,
        F: Fn(HttpResponse) -> Ev + Send + 'static,
    {
        let request = HttpRequest {
            method: method.to_string(),
            url: url.to_string(),
        };

        self.sender
            .send(Command::new(request, callback))
    }

    pub fn map_event<F, NewEvent>(&self, f: F) -> Http<NewEvent>
    where
        F: Fn(NewEvent) -> Ev + Send + Sync + Copy + 'static,
        Ev: 'static,
        NewEvent: 'static,
    {
        Http::new(self.sender.map_event(f))
    }
}
