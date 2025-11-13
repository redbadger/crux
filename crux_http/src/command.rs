//! The Command based API for `crux_http`
//!
//! Use methods on the [`Http`] type. For example:
//!
//! ```
//! # use crux_core::macros::effect;
//! # use crux_http::HttpRequest;
//! # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<String>>) }
//! # #[effect]
//! # #[allow(unused)]
//! # enum Effect { Http(HttpRequest) }
//! # type Http = crux_http::command::Http<Effect, Event>;
//! Http::get("https://httpbin.org/get")
//!     .expect_string()
//!     .build()
//!     .then_send(Event::ReceiveResponse);
//! ```
//!
//!

use std::{fmt, future::Future, marker::PhantomData};

use crux_core::{Command, command};
use http_types::{
    Body, Method, Mime, Url,
    convert::DeserializeOwned,
    headers::{HeaderName, ToHeaderValues},
};
use serde::Serialize;

use crate::{
    HttpError, Request, Response,
    expect::{ExpectBytes, ExpectJson, ExpectString, ResponseExpectation},
    middleware::Middleware,
    protocol::{HttpRequest, HttpResult, ProtocolRequestBuilder},
};

#[deprecated(since = "0.16.0", note = "Import directly from crate root")]
pub use crate::Http;

/// Request Builder
///
/// Provides an ergonomic way to chain the creation of a request.
/// This is generally accessed as the return value from
/// `crux_http::command::Http::{method}()`.
///
/// # Examples
///
/// ```
/// # use crux_core::macros::effect;
/// # use crux_http::HttpRequest;
/// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
/// # #[effect]
/// # #[allow(unused)]
/// # enum Effect { Http(HttpRequest) }
/// # type Http = crux_http::command::Http<Effect, Event>;
/// Http::post("https://httpbin.org/post")
///     .body("<html>hi</html>")
///     .header("custom-header", "value")
///     .content_type(crux_http::http::mime::HTML)
///     .build()
///     .then_send(Event::ReceiveResponse);
/// ```
#[must_use]
pub struct RequestBuilder<Effect, Event, ExpectBody = Vec<u8>> {
    /// Holds the state of the request.
    req: Option<Request>,
    effect: PhantomData<Effect>,
    event: PhantomData<fn() -> Event>,
    expectation: Box<dyn ResponseExpectation<Body = ExpectBody> + Send>,
}

impl<Effect, Event> RequestBuilder<Effect, Event, Vec<u8>>
where
    Effect: Send + From<crux_core::Request<HttpRequest>> + 'static,
    Event: 'static,
{
    pub(crate) fn new(method: Method, url: Url) -> Self {
        Self {
            req: Some(Request::new(method, url)),
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
    /// Sets a header on the request.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crux_core::macros::effect;
    /// # use crux_http::HttpRequest;
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[effect]
    /// # #[allow(unused)]
    /// # enum Effect { Http(HttpRequest) }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::get("https://httpbin.org/get")
    ///     .body("<html>hi</html>")
    ///     .header("header-name", "header-value")
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    /// ```
    ///
    #[allow(clippy::missing_panics_doc)]
    pub fn header(mut self, key: impl Into<HeaderName>, value: impl ToHeaderValues) -> Self {
        self.req.as_mut().unwrap().insert_header(key, value);
        self
    }

    /// Sets the Content-Type header on the request.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crux_core::macros::effect;
    /// # use crux_http::HttpRequest;
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[effect]
    /// # #[allow(unused)]
    /// # enum Effect { Http(HttpRequest) }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::get("https://httpbin.org/get")
    ///     .content_type(crux_http::http::mime::HTML)
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    /// ```
    #[allow(clippy::missing_panics_doc)]
    pub fn content_type(mut self, content_type: impl Into<Mime>) -> Self {
        self.req
            .as_mut()
            .unwrap()
            .set_content_type(content_type.into());
        self
    }

    /// Sets the body of the request from any type that implements `Into<Body>`
    ///
    /// # Mime
    ///
    /// The encoding is set to `application/octet-stream`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crux_core::macros::effect;
    /// # use crux_http::HttpRequest;
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[effect]
    /// # #[allow(unused)]
    /// # enum Effect { Http(HttpRequest) }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::post("https://httpbin.org/post")
    ///     .body(serde_json::json!({"any": "Into<Body>"}))
    ///     .content_type(crux_http::http::mime::HTML)
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    /// ```
    #[allow(clippy::missing_panics_doc)]
    pub fn body(mut self, body: impl Into<Body>) -> Self {
        self.req.as_mut().unwrap().set_body(body);
        self
    }

    /// Pass JSON as the request body.
    ///
    /// # Mime
    ///
    /// The encoding is set to `application/json`.
    ///
    /// # Errors
    ///
    /// This method will return an error if the provided data could not be serialized to JSON.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde::{Deserialize, Serialize};
    /// # use crux_core::macros::effect;
    /// # use crux_http::HttpRequest;
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[effect]
    /// # #[allow(unused)]
    /// # enum Effect { Http(HttpRequest) }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// #[derive(Deserialize, Serialize)]
    /// struct Ip {
    ///     ip: String
    /// }
    ///
    /// let data = &Ip { ip: "129.0.0.1".into() };
    /// Http::post("https://httpbin.org/post")
    ///     .body_json(data)
    ///     .expect("could not serialize body")
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    /// ```
    pub fn body_json(self, json: &impl Serialize) -> crate::Result<Self> {
        Ok(self.body(Body::from_json(json)?))
    }

    /// Pass a string as the request body.
    ///
    /// # Mime
    ///
    /// The encoding is set to `text/plain; charset=utf-8`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crux_core::macros::effect;
    /// # use crux_http::HttpRequest;
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[effect]
    /// # #[allow(unused)]
    /// # enum Effect { Http(HttpRequest) }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::post("https://httpbin.org/post")
    ///     .body_string("hello_world".to_string())
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    /// ```
    pub fn body_string(self, string: String) -> Self {
        self.body(Body::from_string(string))
    }

    /// Pass bytes as the request body.
    ///
    /// # Mime
    ///
    /// The encoding is set to `application/octet-stream`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crux_core::macros::effect;
    /// # use crux_http::HttpRequest;
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[effect]
    /// # #[allow(unused)]
    /// # enum Effect { Http(HttpRequest) }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::post("https://httpbin.org/post")
    ///     .body_bytes(b"hello_world".to_owned())
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    /// ```
    pub fn body_bytes(self, bytes: impl AsRef<[u8]>) -> Self {
        self.body(Body::from(bytes.as_ref()))
    }

    /// Pass form data as the request body. The form data needs to be
    /// serializable to name-value pairs.
    ///
    /// # Mime
    ///
    /// The `content-type` is set to `application/x-www-form-urlencoded`.
    ///
    /// # Errors
    ///
    /// An error will be returned if the provided data cannot be serialized to
    /// form data.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::collections::HashMap;
    /// # use crux_core::macros::effect;
    /// # use crux_http::HttpRequest;
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[effect]
    /// # #[allow(unused)]
    /// # enum Effect { Http(HttpRequest) }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// let form_data = HashMap::from([
    ///     ("name", "Alice"),
    ///     ("location", "UK"),
    /// ]);
    /// Http::post("https://httpbin.org/post")
    ///     .body_form(&form_data)
    ///     .expect("could not serialize body")
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    /// ```
    pub fn body_form(self, form: &impl Serialize) -> crate::Result<Self> {
        Ok(self.body(Body::from_form(form)?))
    }

    /// Set the URL querystring.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde::{Deserialize, Serialize};
    /// # use crux_core::macros::effect;
    /// # use crux_http::HttpRequest;
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[effect]
    /// # #[allow(unused)]
    /// # enum Effect { Http(HttpRequest) }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// #[derive(Serialize, Deserialize)]
    /// struct Index {
    ///     page: u32
    /// }
    ///
    /// let query = Index { page: 2 };
    /// Http::post("https://httpbin.org/post")
    ///     .query(&query)
    ///     .expect("could not serialize query string")
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    /// ```
    ///
    /// # Errors
    /// Returns an error if the query string could not be serialized.
    #[allow(clippy::missing_panics_doc)]
    pub fn query(mut self, query: &impl Serialize) -> std::result::Result<Self, HttpError> {
        self.req.as_mut().unwrap().set_query(query)?;

        Ok(self)
    }

    /// Push middleware onto a per-request middleware stack.
    ///
    /// **Important**: Setting per-request middleware incurs extra allocations.
    /// Creating a `Client` with middleware is recommended.
    ///
    /// Client middleware is run before per-request middleware.
    ///
    /// See the [middleware] submodule for more information on middleware.
    ///
    /// [middleware]: ../middleware/index.html
    ///
    /// # Examples
    ///
    /// ```
    /// # use crux_core::macros::effect;
    /// # use crux_http::HttpRequest;
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[effect]
    /// # #[allow(unused)]
    /// # enum Effect { Http(HttpRequest) }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::get("https://httpbin.org/redirect/2")
    ///     .middleware(crux_http::middleware::Redirect::default())
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    /// ```
    ///
    #[allow(clippy::missing_panics_doc)]
    pub fn middleware(mut self, middleware: impl Middleware) -> Self {
        self.req.as_mut().unwrap().middleware(middleware);
        self
    }

    /// Return the constructed `Request` in a [`crux_core::command::RequestBuilder`].
    ///
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn build(
        self,
    ) -> command::RequestBuilder<
        Effect,
        Event,
        impl Future<Output = Result<Response<ExpectBody>, HttpError>>,
    > {
        let req = self.req.expect("RequestBuilder::build called twice");

        command::RequestBuilder::new(|ctx| async move {
            let operation = req
                .into_protocol_request()
                .await
                .expect("should be able to convert request to protocol request");

            let result = Command::request_from_shell(operation)
                .into_future(ctx)
                .await;

            match result {
                HttpResult::Ok(response) => Response::<Vec<u8>>::new(response.into())
                    .await
                    .and_then(|r| self.expectation.decode(r)),
                HttpResult::Err(error) => Err(error),
            }
        })
    }

    /// Decode a String from the response body prior to dispatching it to the apps `update`
    /// function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crux_core::macros::effect;
    /// # use crux_http::HttpRequest;
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<String>>) }
    /// # #[effect]
    /// # #[allow(unused)]
    /// # enum Effect { Http(HttpRequest) }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::post("https://httpbin.org/json")
    ///     .expect_string()
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    /// ```
    pub fn expect_string(self) -> RequestBuilder<Effect, Event, String> {
        let expectation = Box::<ExpectString>::default();
        RequestBuilder {
            req: self.req,
            effect: PhantomData,
            event: PhantomData,
            expectation,
        }
    }

    /// Decode a `T` from a JSON response body prior to dispatching it to the apps `update`
    /// function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde::{Deserialize, Serialize};
    /// # use crux_core::macros::effect;
    /// # use crux_http::HttpRequest;
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Slideshow>>) }
    /// # #[effect]
    /// # #[allow(unused)]
    /// # enum Effect { Http(HttpRequest) }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// #[derive(Deserialize)]
    /// struct Response {
    ///     slideshow: Slideshow
    /// }
    ///
    /// #[derive(Deserialize)]
    /// struct Slideshow {
    ///     author: String
    /// }
    ///
    /// Http::post("https://httpbin.org/json")
    ///     .expect_json::<Slideshow>()
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    /// ```
    pub fn expect_json<T>(self) -> RequestBuilder<Effect, Event, T>
    where
        T: DeserializeOwned + 'static,
    {
        let expectation = Box::<ExpectJson<T>>::default();
        RequestBuilder {
            req: self.req,
            effect: PhantomData,
            event: PhantomData,
            expectation,
        }
    }
}

impl<Effect, Event> fmt::Debug for RequestBuilder<Effect, Event> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.req, f)
    }
}
