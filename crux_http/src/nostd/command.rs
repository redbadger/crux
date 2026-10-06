//! spike(no_std): the Command based API for `crux_http`, without std.
//!
//! The std `RequestBuilder` wraps a `Request` built on `http` types, which cannot be
//! built without std. This one builds the protocol [`HttpRequest`] directly. Calls that
//! compile against both:
//!
//! ```ignore
//! Http::get(url).expect_json().build().then_send(Event::Set)
//! Http::post(url).header("x-api-key", key).body_json(&value)?.build().then_send(Event::Set)
//! ```
//!
//! Not available here: `content_type(Mime)`, `body(impl Into<Body>)`, `body_form`, `query`,
//! `middleware`, and `Http::request(Method, Url)` (it takes a method string instead).
//!
//! What goes to the shell matches the std builder: header names are lowercased (as
//! `http::HeaderName` does), `header` replaces an existing header of the same name, each
//! `body_*` sets `content-type` the same way, and the URL is normalised by [`Url`].

use core::{fmt, future::Future, marker::PhantomData};

use crux_core::{Command, command};
use serde::Serialize;
use serde::de::DeserializeOwned;
use url::Url;

#[allow(unused_imports)]
use crate::prelude::*;
use crate::{
    Response, Result,
    expect::{ExpectBytes, ExpectJson, ExpectString, ResponseExpectation},
    protocol::{HttpHeader, HttpRequest, HttpResult},
};

#[deprecated(since = "0.16.0", note = "Import directly from crate root")]
pub use crate::Http;

macro_rules! http_method {
    ($(#[$doc:meta])* $name:ident, $method:literal) => {
        $(#[$doc])*
        /// # Panics
        ///
        /// This will panic if a malformed URL is passed.
        pub fn $name(url: impl AsRef<str>) -> RequestBuilder<Effect, Event> {
            RequestBuilder::new($method, url.as_ref())
        }
    };
}

impl<Effect, Event> Http<Effect, Event>
where
    Effect: Send + From<crux_core::Request<HttpRequest>> + 'static,
    Event: Send + 'static,
{
    http_method!(
        /// Instruct the Shell to perform a HTTP GET request to the provided `url`.
        get, "GET"
    );
    http_method!(
        /// Instruct the Shell to perform a HTTP HEAD request to the provided `url`.
        head, "HEAD"
    );
    http_method!(
        /// Instruct the Shell to perform a HTTP POST request to the provided `url`.
        post, "POST"
    );
    http_method!(
        /// Instruct the Shell to perform a HTTP PUT request to the provided `url`.
        put, "PUT"
    );
    http_method!(
        /// Instruct the Shell to perform a HTTP DELETE request to the provided `url`.
        delete, "DELETE"
    );
    http_method!(
        /// Instruct the Shell to perform a HTTP PATCH request to the provided `url`.
        patch, "PATCH"
    );
    http_method!(
        /// Instruct the Shell to perform a HTTP OPTIONS request to the provided `url`.
        options, "OPTIONS"
    );
    http_method!(
        /// Instruct the Shell to perform a HTTP TRACE request to the provided `url`.
        trace, "TRACE"
    );
    http_method!(
        /// Instruct the Shell to perform a HTTP CONNECT request to the provided `url`.
        connect, "CONNECT"
    );

    /// Instruct the Shell to perform an HTTP request with any method.
    ///
    /// spike(no_std): the std version takes an `http::Method`; this takes its name.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    pub fn request(method: impl AsRef<str>, url: impl AsRef<str>) -> RequestBuilder<Effect, Event> {
        RequestBuilder::new(method.as_ref(), url.as_ref())
    }
}

/// Request Builder
///
/// Configures a request and turns it into a [`Command`] with [`RequestBuilder::build`].
/// Usually reached through `Http::{method}()`.
#[must_use]
pub struct RequestBuilder<Effect, Event, ExpectBody = Vec<u8>> {
    req: Option<HttpRequest>,
    effect: PhantomData<Effect>,
    event: PhantomData<fn() -> Event>,
    expectation: Box<dyn ResponseExpectation<Body = ExpectBody> + Send>,
}

impl<Effect, Event> RequestBuilder<Effect, Event, Vec<u8>>
where
    Effect: Send + From<crux_core::Request<HttpRequest>> + 'static,
    Event: 'static,
{
    pub(crate) fn new(method: &str, url: &str) -> Self {
        let url: Url = url.parse().unwrap();
        Self {
            req: Some(HttpRequest {
                method: method.to_string(),
                url: url.into(),
                headers: Vec::new(),
                body: Vec::new(),
            }),
            effect: PhantomData,
            event: PhantomData,
            expectation: Box::new(ExpectBytes),
        }
    }
}

impl<Effect, Event, ExpectBody> RequestBuilder<Effect, Event, ExpectBody>
where
    Effect: Send + From<crux_core::Request<HttpRequest>> + 'static,
    Event: Send + 'static,
    ExpectBody: 'static,
{
    /// Sets a header on the request, replacing any header of the same name.
    #[allow(clippy::missing_panics_doc)]
    pub fn header(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.req
            .as_mut()
            .unwrap()
            .set_header(name.as_ref(), value.as_ref());
        self
    }

    /// Sets the body to the given bytes, with `content-type: application/octet-stream`.
    pub fn body_bytes(self, bytes: impl AsRef<[u8]>) -> Self {
        self.body(bytes.as_ref().to_vec(), "application/octet-stream")
    }

    /// Sets the body to the given string, with `content-type: text/plain; charset=utf-8`.
    pub fn body_string(self, string: String) -> Self {
        self.body(string.into_bytes(), "text/plain; charset=utf-8")
    }

    /// Sets the body to `json` serialized, with `content-type: application/json`.
    ///
    /// # Errors
    /// [`HttpError::Json`](crate::HttpError::Json) if `json` cannot be serialized.
    pub fn body_json(self, json: &impl Serialize) -> Result<Self> {
        let bytes = serde_json::to_vec(json)?;
        Ok(self.body(bytes, "application/json"))
    }

    #[allow(clippy::missing_panics_doc)]
    fn body(mut self, body: Vec<u8>, content_type: &str) -> Self {
        let req = self.req.as_mut().unwrap();
        req.set_header("content-type", content_type);
        req.body = body;
        self
    }

    /// Turns the request into a [`Command`] builder whose output is the response.
    ///
    /// # Panics
    /// If called twice.
    #[must_use]
    pub fn build(
        self,
    ) -> command::RequestBuilder<Effect, Event, impl Future<Output = Result<Response<ExpectBody>>>>
    {
        let req = self.req.expect("RequestBuilder::build called twice");

        command::RequestBuilder::new(|ctx| async move {
            let result = Command::request_from_shell(req).into_future(ctx).await;

            match result {
                HttpResult::Ok(response) => Response::<Vec<u8>>::new(response)
                    .and_then(|response| self.expectation.decode(response)),
                HttpResult::Err(error) => Err(error),
            }
        })
    }

    /// Decode the response body as a UTF-8 string.
    pub fn expect_string(self) -> RequestBuilder<Effect, Event, String> {
        RequestBuilder {
            req: self.req,
            effect: PhantomData,
            event: PhantomData,
            expectation: Box::<ExpectString>::default(),
        }
    }

    /// Decode the response body from JSON into `T`.
    pub fn expect_json<T>(self) -> RequestBuilder<Effect, Event, T>
    where
        T: DeserializeOwned + 'static,
    {
        RequestBuilder {
            req: self.req,
            effect: PhantomData,
            event: PhantomData,
            expectation: Box::<ExpectJson<T>>::default(),
        }
    }
}

impl<Effect, Event> fmt::Debug for RequestBuilder<Effect, Event> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.req, f)
    }
}

impl HttpRequest {
    /// Sets a header, lowercasing the name and replacing any header of the same name.
    fn set_header(&mut self, name: &str, value: &str) {
        let name = name.to_ascii_lowercase();
        self.headers.retain(|header| header.name != name);
        self.headers.push(HttpHeader {
            name,
            value: value.to_string(),
        });
    }
}
