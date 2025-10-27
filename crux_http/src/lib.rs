#![deny(clippy::pedantic)]
//! A HTTP client for use with Crux
//!
//! `crux_http` allows Crux apps to make HTTP requests by asking the Shell to perform them.
//!
//! This is still work in progress and large parts of HTTP are not yet supported.
// #![warn(missing_docs)]

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

use std::marker::PhantomData;

pub use http_types as http;

use http_types::Method;
use url::Url;

use crate::protocol::HttpRequest;

pub use self::{config::Config, error::HttpError, request::Request};
pub use response::Response;

use request_builder::RequestBuilder;
use response::ResponseAsync;

use client::Client;

pub type Result<T> = std::result::Result<T, HttpError>;

pub struct Http<Effect, Event> {
    effect: PhantomData<Effect>,
    event: PhantomData<Event>,
}

impl<Effect, Event> Http<Effect, Event>
where
    Effect: Send + From<crux_core::Request<HttpRequest>> + 'static,
    Event: Send + 'static,
{
    /// Instruct the Shell to perform a HTTP GET request to the provided `url`.
    ///
    /// The request can be configured via associated functions on the returned
    /// [`RequestBuilder`] and then converted to a [`Command`]
    /// with [`RequestBuilder::build`].
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Examples
    ///
    /// ```
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<String>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::get("https://httpbin.org/get")
    ///     .expect_string()
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    /// ```
    pub fn get(url: impl AsRef<str>) -> command::RequestBuilder<Effect, Event> {
        command::RequestBuilder::new(Method::Get, url.as_ref().parse().unwrap())
    }

    /// Instruct the Shell to perform a HTTP HEAD request to the provided `url`.
    ///
    /// The request can be configured via associated functions on the returned
    /// [`RequestBuilder`] and then converted to a [`Command`]
    /// with [`RequestBuilder::build`].
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Examples
    ///
    /// ```
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::head("https://httpbin.org/get")
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    pub fn head(url: impl AsRef<str>) -> command::RequestBuilder<Effect, Event> {
        command::RequestBuilder::new(Method::Head, url.as_ref().parse().unwrap())
    }

    /// Instruct the Shell to perform a HTTP POST request to the provided `url`.
    ///
    /// The request can be configured via associated functions on the returned
    /// [`RequestBuilder`] and then converted to a [`Command`]
    /// with [`RequestBuilder::build`].
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Examples
    ///
    /// ```
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::post("https://httpbin.org/post")
    ///     .body_bytes(b"hello_world".to_owned())
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    pub fn post(url: impl AsRef<str>) -> command::RequestBuilder<Effect, Event> {
        command::RequestBuilder::new(Method::Post, url.as_ref().parse().unwrap())
    }

    /// Instruct the Shell to perform a HTTP PUT request to the provided `url`.
    ///
    /// The request can be configured via associated functions on the returned
    /// [`RequestBuilder`] and then converted to a [`Command`]
    /// with [`RequestBuilder::build`].
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Examples
    ///
    /// ```
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::put("https://httpbin.org/put")
    ///     .body_string("hello_world".to_string())
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    pub fn put(url: impl AsRef<str>) -> command::RequestBuilder<Effect, Event> {
        command::RequestBuilder::new(Method::Put, url.as_ref().parse().unwrap())
    }

    /// Instruct the Shell to perform a HTTP DELETE request to the provided `url`.
    ///
    /// The request can be configured via associated functions on the returned
    /// [`RequestBuilder`] and then converted to a [`Command`]
    /// with [`RequestBuilder::build`].
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Examples
    ///
    /// ```
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::delete("https://httpbin.org/delete")
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    pub fn delete(url: impl AsRef<str>) -> command::RequestBuilder<Effect, Event> {
        command::RequestBuilder::new(Method::Delete, url.as_ref().parse().unwrap())
    }

    /// Instruct the Shell to perform a HTTP PATCH request to the provided `url`.
    ///
    /// The request can be configured via associated functions on the returned
    /// [`RequestBuilder`] and then converted to a [`Command`]
    /// with [`RequestBuilder::build`].
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Examples
    ///
    /// ```
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::patch("https://httpbin.org/patch")
    ///     .body_form(&[("name", "Alice")]).unwrap()
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    pub fn patch(url: impl AsRef<str>) -> command::RequestBuilder<Effect, Event> {
        command::RequestBuilder::new(Method::Patch, url.as_ref().parse().unwrap())
    }

    /// Instruct the Shell to perform a HTTP OPTIONS request to the provided `url`.
    ///
    /// The request can be configured via associated functions on the returned
    /// [`RequestBuilder`] and then converted to a [`Command`]
    /// with [`RequestBuilder::build`].
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Examples
    ///
    /// ```
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::options("https://httpbin.org/get")
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    pub fn options(url: impl AsRef<str>) -> command::RequestBuilder<Effect, Event> {
        command::RequestBuilder::new(Method::Options, url.as_ref().parse().unwrap())
    }

    /// Instruct the Shell to perform a HTTP TRACE request to the provided `url`.
    ///
    /// The request can be configured via associated functions on the returned
    /// [`RequestBuilder`] and then converted to a [`Command`]
    /// with [`RequestBuilder::build`].
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Examples
    ///
    /// ```
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::trace("https://httpbin.org/get")
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    pub fn trace(url: impl AsRef<str>) -> command::RequestBuilder<Effect, Event> {
        command::RequestBuilder::new(Method::Trace, url.as_ref().parse().unwrap())
    }

    /// Instruct the Shell to perform a HTTP CONNECT request to the provided `url`.
    ///
    /// The request can be configured via associated functions on the returned
    /// [`RequestBuilder`] and then converted to a [`Command`]
    /// with [`RequestBuilder::build`].
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Examples
    ///
    /// ```
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::connect("https://httpbin.org/get")
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    pub fn connect(url: impl AsRef<str>) -> command::RequestBuilder<Effect, Event> {
        command::RequestBuilder::new(Method::Connect, url.as_ref().parse().unwrap())
    }

    /// Instruct the Shell to perform an HTTP request to the provided `url`.
    ///
    /// The request can be configured via associated functions on the returned
    /// [`RequestBuilder`] and then converted to a [`Command`]
    /// with [`RequestBuilder::build`].
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use http_types::Method;
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::request(Method::Post, "https://httpbin.org/post".parse().unwrap())
    ///     .body_form(&[("name", "Alice")]).unwrap()
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    pub fn request(method: Method, url: Url) -> command::RequestBuilder<Effect, Event> {
        command::RequestBuilder::new(method, url)
    }
}
