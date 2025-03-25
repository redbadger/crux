//! The Command based API for crux_http

use std::{fmt, future::Future, marker::PhantomData};

use crux_core::{command, Command};
use http_types::{
    convert::DeserializeOwned,
    headers::{HeaderName, ToHeaderValues},
    Body, Method, Mime, Url,
};
use serde::Serialize;

use crate::{
    expect::{ExpectBytes, ExpectJson, ExpectString, ResponseExpectation},
    middleware::Middleware,
    protocol::{HttpRequest, HttpResult, ProtocolRequestBuilder},
    HttpError, Request, Response,
};

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
    pub fn get(url: impl AsRef<str>) -> RequestBuilder<Effect, Event> {
        RequestBuilder::new(Method::Get, url.as_ref().parse().unwrap())
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
    pub fn head(url: impl AsRef<str>) -> RequestBuilder<Effect, Event> {
        RequestBuilder::new(Method::Head, url.as_ref().parse().unwrap())
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
    pub fn post(url: impl AsRef<str>) -> RequestBuilder<Effect, Event> {
        RequestBuilder::new(Method::Post, url.as_ref().parse().unwrap())
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
    pub fn put(url: impl AsRef<str>) -> RequestBuilder<Effect, Event> {
        RequestBuilder::new(Method::Put, url.as_ref().parse().unwrap())
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
    pub fn delete(url: impl AsRef<str>) -> RequestBuilder<Effect, Event> {
        RequestBuilder::new(Method::Delete, url.as_ref().parse().unwrap())
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
    pub fn patch(url: impl AsRef<str>) -> RequestBuilder<Effect, Event> {
        RequestBuilder::new(Method::Patch, url.as_ref().parse().unwrap())
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
    pub fn options(url: impl AsRef<str>) -> RequestBuilder<Effect, Event> {
        RequestBuilder::new(Method::Options, url.as_ref().parse().unwrap())
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
    pub fn trace(url: impl AsRef<str>) -> RequestBuilder<Effect, Event> {
        RequestBuilder::new(Method::Trace, url.as_ref().parse().unwrap())
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
    pub fn connect(url: impl AsRef<str>) -> RequestBuilder<Effect, Event> {
        RequestBuilder::new(Method::Connect, url.as_ref().parse().unwrap())
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
    pub fn request(method: Method, url: Url) -> RequestBuilder<Effect, Event> {
        RequestBuilder::new(method, url)
    }
}

/// Request Builder
///
/// Provides an ergonomic way to chain the creation of a request.
/// This is generally accessed as the return value from
/// `crux_http::command::Http::{method}()`.
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
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::get("https://httpbin.org/get")
    ///     .body("<html>hi</html>")
    ///     .header("header-name", "header-value")
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    /// ```
    pub fn header(mut self, key: impl Into<HeaderName>, value: impl ToHeaderValues) -> Self {
        self.req.as_mut().unwrap().insert_header(key, value);
        self
    }

    /// Sets the Content-Type header on the request.
    ///
    /// # Examples
    ///
    /// ```
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::get("https://httpbin.org/get")
    ///     .content_type(crux_http::http::mime::HTML)
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    /// ```
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
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::post("https://httpbin.org/post")
    ///     .body(serde_json::json!({"any": "Into<Body>"}))
    ///     .content_type(crux_http::http::mime::HTML)
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    /// ```
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
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
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
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
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
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
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
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
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
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
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
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # type Http = crux_http::command::Http<Effect, Event>;
    /// Http::get("https://httpbin.org/redirect/2")
    ///     .middleware(crux_http::middleware::Redirect::default())
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    /// ```
    pub fn middleware(mut self, middleware: impl Middleware) -> Self {
        self.req.as_mut().unwrap().middleware(middleware);
        self
    }

    /// Return the constructed `Request` in a [`crux_core::command::RequestBuilder`].
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
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<String>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
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
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Slideshow>>) }
    /// # #[derive(crux_core::macros::Effect)]
    /// # #[allow(unused)]
    /// # struct Capabilities { http: crux_http::Http<Event> }
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
