#![deny(clippy::pedantic)]
//! A HTTP client for use with Crux
//!
//! `crux_http` allows Crux apps to make HTTP requests by asking the Shell to perform them.
//!
//! This is still work in progress and large parts of HTTP are not yet supported.
// #![warn(missing_docs)]

#[expect(deprecated)]
use crux_core::capability::CapabilityContext;

mod config;
mod error;
mod expect;
mod request;
mod request_builder;
mod response;

pub mod client;
pub mod command;
pub mod middleware;
pub mod protocol;
pub mod testing;

pub use http::{Method, StatusCode, Version, header};
pub use mime::Mime;
pub use url::Url;

pub use self::{
    config::Config,
    error::HttpError,
    request::Request,
    request_builder::RequestBuilder,
    response::{Response, ResponseAsync},
};

use client::Client;

pub type Result<T> = std::result::Result<T, HttpError>;

/// The original Http capability API, now deprecated.
#[deprecated(
    since = "0.15.0",
    note = "The capabilities API has been deprecated. Use command::Http instead."
)]
pub struct Http<Ev> {
    #[expect(deprecated)]
    context: CapabilityContext<protocol::HttpRequest, Ev>,
    client: Client,
}

#[expect(deprecated)]
impl<Ev> crux_core::Capability<Ev> for Http<Ev> {
    type Operation = protocol::HttpRequest;

    type MappedSelf<MappedEv> = Http<MappedEv>;

    fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
    where
        F: Fn(NewEv) -> Ev + Send + Sync + 'static,
        Ev: 'static,
        NewEv: 'static + Send,
    {
        Http::new(self.context.map_event(f))
    }
}

#[expect(deprecated)]
impl<Ev> Clone for Http<Ev> {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
            client: self.client.clone(),
        }
    }
}

#[expect(deprecated)]
impl<Ev> Http<Ev>
where
    Ev: 'static,
{
    #[must_use]
    pub fn new(context: CapabilityContext<protocol::HttpRequest, Ev>) -> Self {
        Self {
            client: Client::new(context.clone()),
            context,
        }
    }

    /// Instruct the Shell to perform a HTTP GET request to the provided `url`.
    ///
    /// The request can be configured via associated functions on `RequestBuilder`
    /// and then sent with `RequestBuilder::send`
    ///
    /// When finished, the response will be wrapped in an event and dispatched to
    /// the app's `update` function.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) {
    /// caps.http.get("https://httpbin.org/get").send(Event::ReceiveResponse)
    /// # }
    /// ```
    pub fn get(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::GET, url.as_ref().parse().unwrap(), self.clone())
    }

    /// Instruct the Shell to perform a HTTP HEAD request to the provided `url`.
    ///
    /// The request can be configured via associated functions on `RequestBuilder`
    /// and then sent with `RequestBuilder::send`
    ///
    /// When finished, the response will be wrapped in an event and dispatched to
    /// the app's `update` function.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) {
    /// caps.http.head("https://httpbin.org/get").send(Event::ReceiveResponse)
    /// # }
    /// ```
    pub fn head(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::HEAD, url.as_ref().parse().unwrap(), self.clone())
    }

    /// Instruct the Shell to perform a HTTP POST request to the provided `url`.
    ///
    /// The request can be configured via associated functions on `RequestBuilder`
    /// and then sent with `RequestBuilder::send`
    ///
    /// When finished, the response will be wrapped in an event and dispatched to
    /// the app's `update` function.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) {
    /// caps.http.post("https://httpbin.org/post").send(Event::ReceiveResponse)
    /// # }
    /// ```
    pub fn post(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::POST, url.as_ref().parse().unwrap(), self.clone())
    }

    /// Instruct the Shell to perform a HTTP PUT request to the provided `url`.
    ///
    /// The request can be configured via associated functions on `RequestBuilder`
    /// and then sent with `RequestBuilder::send`
    ///
    /// When finished, the response will be wrapped in an event and dispatched to
    /// the app's `update` function.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) {
    /// caps.http.put("https://httpbin.org/post").send(Event::ReceiveResponse)
    /// # }
    /// ```
    pub fn put(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::PUT, url.as_ref().parse().unwrap(), self.clone())
    }

    /// Instruct the Shell to perform a HTTP DELETE request to the provided `url`.
    ///
    /// The request can be configured via associated functions on `RequestBuilder`
    /// and then sent with `RequestBuilder::send`
    ///
    /// When finished, the response will be wrapped in an event and dispatched to
    /// the app's `update` function.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) {
    /// caps.http.delete("https://httpbin.org/post").send(Event::ReceiveResponse)
    /// # }
    /// ```
    pub fn delete(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::DELETE, url.as_ref().parse().unwrap(), self.clone())
    }

    /// Instruct the Shell to perform a HTTP CONNECT request to the provided `url`.
    ///
    /// The request can be configured via associated functions on `RequestBuilder`
    /// and then sent with `RequestBuilder::send`
    ///
    /// When finished, the response will be wrapped in an event and dispatched to
    /// the app's `update` function.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) {
    /// caps.http.connect("https://httpbin.org/get").send(Event::ReceiveResponse)
    /// # }
    /// ```
    pub fn connect(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::CONNECT, url.as_ref().parse().unwrap(), self.clone())
    }

    /// Instruct the Shell to perform a HTTP OPTIONS request to the provided `url`.
    ///
    /// The request can be configured via associated functions on `RequestBuilder`
    /// and then sent with `RequestBuilder::send`
    ///
    /// When finished, the response will be wrapped in an event and dispatched to
    /// the app's `update` function.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) {
    /// caps.http.options("https://httpbin.org/get").send(Event::ReceiveResponse)
    /// # }
    /// ```
    pub fn options(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::OPTIONS, url.as_ref().parse().unwrap(), self.clone())
    }

    /// Instruct the Shell to perform a HTTP TRACE request to the provided `url`.
    ///
    /// The request can be configured via associated functions on `RequestBuilder`
    /// and then sent with `RequestBuilder::send`
    ///
    /// When finished, the response will be wrapped in an event and dispatched to
    /// the app's `update` function.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) {
    /// caps.http.trace("https://httpbin.org/get").send(Event::ReceiveResponse)
    /// # }
    /// ```
    pub fn trace(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::TRACE, url.as_ref().parse().unwrap(), self.clone())
    }

    /// Instruct the Shell to perform a HTTP PATCH request to the provided `url`.
    ///
    /// The request can be configured via associated functions on `RequestBuilder`
    /// and then sent with `RequestBuilder::send`
    ///
    /// When finished, the response will be wrapped in an event and dispatched to
    /// the app's `update` function.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    pub fn patch(&self, url: impl AsRef<str>) -> RequestBuilder<Ev> {
        RequestBuilder::new(Method::PATCH, url.as_ref().parse().unwrap(), self.clone())
    }

    /// Instruct the Shell to perform an HTTP request with the provided `method` and `url`.
    ///
    /// The request can be configured via associated functions on `RequestBuilder`
    /// and then sent with `RequestBuilder::send`
    ///
    /// When finished, the response will be wrapped in an event and dispatched to
    /// the app's `update` function.
    pub fn request(&self, method: Method, url: Url) -> RequestBuilder<Ev> {
        RequestBuilder::new(method, url, self.clone())
    }
}
