use crate::expect::{ExpectBytes, ExpectJson, ExpectString};
use crate::middleware::Middleware;
use crate::{
    expect::ResponseExpectation,
    http::{
        headers::{HeaderName, ToHeaderValues},
        Body, Method, Mime, Url,
    },
};
use crate::{Client, HttpError, Request, Response, ResponseAsync, Result};

use crux_core::Command;
use futures_util::future::BoxFuture;
use http_types::convert::DeserializeOwned;
use serde::Serialize;

use std::fmt;

/// Request Builder
///
/// Provides an ergonomic way to chain the creation of a request.
/// This is generally accessed as the return value from `Http::{method}()`.
///
/// # Examples
///
/// ```no_run
/// use crux_http::http::{mime::HTML};
/// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
/// # struct Capabilities { http: crux_http::Http }
/// # fn update(caps: &Capabilities) -> crux_core::Command<Event> {
/// caps.http
///     .post("https://httpbin.org/post")
///     .body("<html>hi</html>")
///     .header("custom-header", "value")
///     .content_type(HTML)
///     .send_and_respond(Event::ReceiveResponse)
/// # }
/// ```
#[must_use]
pub struct RequestBuilder<ExpectBody = Vec<u8>> {
    /// Holds the state of the request.
    req: Option<Request>,
    cap_or_client: CapOrClient,
    expectation: Box<dyn ResponseExpectation<Body = ExpectBody> + Send>,
}

// Middleware request builders won't have access to the capability, so they get a client
// and therefore can't send events themselves.  Normal request builders get direct access
// to the capability itself.
enum CapOrClient {
    Client(Client),
    Capability(crate::Http),
}

impl RequestBuilder<Vec<u8>> {
    pub(crate) fn new(method: Method, url: Url, capability: crate::Http) -> Self {
        Self {
            req: Some(Request::new(method, url)),
            cap_or_client: CapOrClient::Capability(capability),
            expectation: Box::new(ExpectBytes),
        }
    }
}

impl RequestBuilder<Vec<u8>> {
    pub(crate) fn new_for_middleware(method: Method, url: Url, client: Client) -> Self {
        Self {
            req: Some(Request::new(method, url)),
            cap_or_client: CapOrClient::Client(client),
            expectation: Box::new(ExpectBytes),
        }
    }
}

impl<ExpectBody> RequestBuilder<ExpectBody>
where
    ExpectBody: 'static,
{
    /// Sets a header on the request.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http }
    /// # fn update(caps: &Capabilities) -> crux_core::Command<Event> {
    /// caps.http
    ///     .get("https://httpbin.org/get")
    ///     .body("<html>hi</html>")
    ///     .header("header-name", "header-value")
    ///     .send_and_respond(Event::ReceiveResponse)
    /// # }
    /// ```
    pub fn header(mut self, key: impl Into<HeaderName>, value: impl ToHeaderValues) -> Self {
        self.req.as_mut().unwrap().insert_header(key, value);
        self
    }

    /// Sets the Content-Type header on the request.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crux_http::http::mime;
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http }
    /// # fn update(caps: &Capabilities) -> crux_core::Command<Event> {
    /// caps.http
    ///     .get("https://httpbin.org/get")
    ///     .content_type(mime::HTML)
    ///     .send_and_respond(Event::ReceiveResponse)
    /// # }
    /// ```
    pub fn content_type(mut self, content_type: impl Into<Mime>) -> Self {
        self.req
            .as_mut()
            .unwrap()
            .set_content_type(content_type.into());
        self
    }

    /// Sets the body of the request from any type with implements `Into<Body>`, for example, any type with is `AsyncRead`.
    /// # Mime
    ///
    /// The encoding is set to `application/octet-stream`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http }
    /// # fn update(caps: &Capabilities) -> crux_core::Command<Event> {
    /// use serde_json::json;
    /// use crux_http::http::mime;
    /// caps.http
    ///     .post("https://httpbin.org/post")
    ///     .body(json!({"any": "Into<Body>"}))
    ///     .content_type(mime::HTML)
    ///     .send_and_respond(Event::ReceiveResponse)
    /// # }
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
    /// ```no_run
    /// # use serde::{Deserialize, Serialize};
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http }
    /// # fn update(caps: &Capabilities) -> crux_core::Command<Event> {
    /// #[derive(Deserialize, Serialize)]
    /// struct Ip {
    ///     ip: String
    /// }
    ///
    /// let data = &Ip { ip: "129.0.0.1".into() };
    /// caps.http
    ///     .post("https://httpbin.org/post")
    ///     .body_json(data)
    ///     .expect("could not serialize body")
    ///     .send_and_respond(Event::ReceiveResponse)
    /// # }
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
    /// ```no_run
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http }
    /// # fn update(caps: &Capabilities) -> crux_core::Command<Event> {
    /// caps.http
    ///     .post("https://httpbin.org/post")
    ///     .body_string("hello_world".to_string())
    ///     .send_and_respond(Event::ReceiveResponse)
    /// # }
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
    /// ```no_run
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http }
    /// # fn update(caps: &Capabilities) -> crux_core::Command<Event> {
    /// caps.http
    ///     .post("https://httpbin.org/post")
    ///     .body_bytes(b"hello_world".to_owned())
    ///     .send_and_respond(Event::ReceiveResponse)
    /// # }
    /// ```
    pub fn body_bytes(self, bytes: impl AsRef<[u8]>) -> Self {
        self.body(Body::from(bytes.as_ref()))
    }

    /// Set the URL querystring.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use serde::{Deserialize, Serialize};
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http }
    /// # fn update(caps: &Capabilities) -> crux_core::Command<Event> {
    /// #[derive(Serialize, Deserialize)]
    /// struct Index {
    ///     page: u32
    /// }
    ///
    /// let query = Index { page: 2 };
    /// caps.http
    ///     .post("https://httpbin.org/post")
    ///     .query(&query)
    ///     .expect("could not serialize query string")
    ///     .send_and_respond(Event::ReceiveResponse)
    /// # }
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
    /// ```no_run
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http }
    /// # fn update(caps: &Capabilities) -> crux_core::Command<Event> {
    ///
    /// caps.http
    ///     .get("https://httpbin.org/redirect/2")
    ///     .middleware(crux_http::middleware::Redirect::default())
    ///     .send_and_respond(Event::ReceiveResponse)
    /// # }
    /// ```
    pub fn middleware(mut self, middleware: impl Middleware) -> Self {
        self.req.as_mut().unwrap().middleware(middleware);
        self
    }

    /// Return the constructed `Request`.
    pub fn build(self) -> Request {
        self.req.unwrap()
    }

    /// Decode a String from the response body prior to dispatching it to the apps `update`
    /// function.
    ///
    /// This has no effect when used with the [async API](RequestBuilder::send_async).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # struct Capabilities { http: crux_http::Http }
    /// enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<String>>) }
    ///
    /// # fn update(caps: &Capabilities) -> crux_core::Command<Event> {
    /// caps.http
    ///     .post("https://httpbin.org/json")
    ///     .expect_string()
    ///     .send_and_respond(Event::ReceiveResponse)
    /// # }
    /// ```
    pub fn expect_string(self) -> RequestBuilder<String> {
        let expectation = Box::<ExpectString>::default();
        RequestBuilder {
            req: self.req,
            cap_or_client: self.cap_or_client,
            expectation,
        }
    }

    /// Decode a `T` from a JSON response body prior to dispatching it to the apps `update`
    /// function.
    ///
    /// This has no effect when used with the [async API](RequestBuilder::send_async).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use serde::{Deserialize, Serialize};
    /// # struct Capabilities { http: crux_http::Http }
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
    /// enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Slideshow>>) }
    ///
    /// # fn update(caps: &Capabilities) -> crux_core::Command<Event> {
    /// caps.http
    ///     .post("https://httpbin.org/json")
    ///     .expect_json::<Slideshow>()
    ///     .send_and_respond(Event::ReceiveResponse)
    /// # }
    /// ```
    pub fn expect_json<T>(self) -> RequestBuilder<T>
    where
        T: DeserializeOwned + 'static,
    {
        let expectation = Box::<ExpectJson<T>>::default();
        RequestBuilder {
            req: self.req,
            cap_or_client: self.cap_or_client,
            expectation,
        }
    }

    /// Sends the constructed `Request` and returns its result as an update `Event`
    ///
    /// When finished, the response will wrapped in an event using `make_event` and
    /// dispatched to the app's `update function.
    pub fn send_and_respond<F, Event>(self, make_event: F) -> Command<Event>
    where
        F: FnOnce(crate::Result<Response<ExpectBody>>) -> Event + Send + 'static,
    {
        let CapOrClient::Capability(capability) = self.cap_or_client else {
            panic!("Called RequestBuilder::send in a middleware context");
        };
        let request = self.req;

        let fut = async move {
            let result = capability.client.send(request.unwrap()).await;

            let resp = match result {
                Ok(resp) => resp,
                Err(e) => {
                    return Command::event(make_event(Err(e)));
                }
            };

            let resp = Response::<Vec<u8>>::new(resp)
                .await
                .and_then(|r| self.expectation.decode(r));

            Command::event(make_event(resp))
        };
        Command::effect(fut)
    }

    /// Sends the constructed `Request` and returns a future that resolves to [`ResponseAsync`].
    /// but does not consume it or convert the body to an expected format.
    ///
    /// Note that this is equivalent to calling `.into_future()` on the `RequestBuilder`, which
    /// will happen implicitly when calling `.await` on the builder, which does implement
    /// [`IntoFuture`](std::future::IntoFuture). Calling `.await` on the builder is recommended.
    ///
    /// Not all code working with futures (such as the `join` macro) works with `IntoFuture` (yet?), so this
    /// method is provided as a more discoverable `.into_future` alias, and may be deprecated later.
    pub fn send(self) -> BoxFuture<'static, Result<ResponseAsync>> {
        <Self as std::future::IntoFuture>::into_future(self)
    }
}

impl<Eb> std::future::IntoFuture for RequestBuilder<Eb> {
    type Output = Result<ResponseAsync>;

    type IntoFuture = BoxFuture<'static, Result<ResponseAsync>>;

    /// Sends the constructed `Request` and returns a future that resolves to the response
    fn into_future(self) -> Self::IntoFuture {
        Box::pin({
            let client = match self.cap_or_client {
                CapOrClient::Client(c) => c,
                CapOrClient::Capability(c) => c.client,
            };

            async move { client.send(self.req.unwrap()).await }
        })
    }
}

impl<Ev> fmt::Debug for RequestBuilder<Ev> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.req, f)
    }
}

// impl From<RequestBuilder<Ev>> for Request {
//     /// Converts a `crux_http::RequestBuilder` to a `crux_http::Request`.
//     fn from(builder: RequestBuilder) -> Request {
//         builder.build()
//     }
// }
