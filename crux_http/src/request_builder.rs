use crate::expect::ResponseExpectation;
use crate::expect::{ExpectBytes, ExpectJson, ExpectString};
use crate::middleware::Middleware;
use crate::{Client, HttpError, Request, Response, ResponseAsync, Result};

use futures_util::future::BoxFuture;
use http_types::{
    convert::DeserializeOwned,
    headers::{HeaderName, ToHeaderValues},
    Body, Method, Mime, Url,
};
use serde::Serialize;

use std::{fmt, marker::PhantomData};

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
/// # struct Capabilities { http: crux_http::Http<Event> }
/// # fn update(caps: &Capabilities) {
/// caps.http
///     .post("https://httpbin.org/post")
///     .body("<html>hi</html>")
///     .header("custom-header", "value")
///     .content_type(HTML)
///     .send(Event::ReceiveResponse)
/// # }
/// ```
#[must_use]
pub struct RequestBuilder<Event, ExpectBody = Vec<u8>> {
    /// Holds the state of the request.
    req: Option<Request>,

    cap_or_client: CapOrClient<Event>,

    phantom: PhantomData<fn() -> Event>,

    expectation: Box<dyn ResponseExpectation<Body = ExpectBody> + Send>,
}

// Middleware request builders won't have access to the capability, so they get a client
// and therefore can't send events themselves.  Normal request builders get direct access
// to the capability itself.
enum CapOrClient<Event> {
    Client(Client),
    Capability(crate::Http<Event>),
}

impl<Event> RequestBuilder<Event, Vec<u8>> {
    pub(crate) fn new(method: Method, url: Url, capability: crate::Http<Event>) -> Self {
        Self {
            req: Some(Request::new(method, url)),
            cap_or_client: CapOrClient::Capability(capability),
            phantom: PhantomData,
            expectation: Box::new(ExpectBytes),
        }
    }
}

impl RequestBuilder<(), Vec<u8>> {
    pub(crate) fn new_for_middleware(method: Method, url: Url, client: Client) -> Self {
        Self {
            req: Some(Request::new(method, url)),
            cap_or_client: CapOrClient::Client(client),
            phantom: PhantomData,
            expectation: Box::new(ExpectBytes),
        }
    }
}

impl<Event, ExpectBody> RequestBuilder<Event, ExpectBody>
where
    Event: 'static,
    ExpectBody: 'static,
{
    /// Sets a header on the request.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) {
    /// caps.http
    ///     .get("https://httpbin.org/get")
    ///     .body("<html>hi</html>")
    ///     .header("header-name", "header-value")
    ///     .send(Event::ReceiveResponse)
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
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) {
    /// caps.http
    ///     .get("https://httpbin.org/get")
    ///     .content_type(mime::HTML)
    ///     .send(Event::ReceiveResponse)
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
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) {
    /// use serde_json::json;
    /// use crux_http::http::mime;
    /// caps.http
    ///     .post("https://httpbin.org/post")
    ///     .body(json!({"any": "Into<Body>"}))
    ///     .content_type(mime::HTML)
    ///     .send(Event::ReceiveResponse)
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
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) {
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
    ///     .send(Event::ReceiveResponse)
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
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) {
    /// caps.http
    ///     .post("https://httpbin.org/post")
    ///     .body_string("hello_world".to_string())
    ///     .send(Event::ReceiveResponse)
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
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) {
    /// caps.http
    ///     .post("https://httpbin.org/post")
    ///     .body_bytes(b"hello_world".to_owned())
    ///     .send(Event::ReceiveResponse)
    /// # }
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
    /// ```no_run
    /// # use std::collections::HashMap;
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) {
    /// let form_data = HashMap::from([
    ///     ("name", "Alice"),
    ///     ("location", "UK"),
    /// ]);
    /// caps.http
    ///     .post("https://httpbin.org/post")
    ///     .body_form(&form_data)
    ///     .expect("could not serialize body")
    ///     .send(Event::ReceiveResponse)
    /// # }
    /// ```
    pub fn body_form(self, form: &impl Serialize) -> crate::Result<Self> {
        Ok(self.body(Body::from_form(form)?))
    }

    /// Set the URL querystring.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use serde::{Deserialize, Serialize};
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) {
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
    ///     .send(Event::ReceiveResponse)
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
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) {
    ///
    /// caps.http
    ///     .get("https://httpbin.org/redirect/2")
    ///     .middleware(crux_http::middleware::Redirect::default())
    ///     .send(Event::ReceiveResponse)
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
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<String>>) }
    ///
    /// # fn update(caps: &Capabilities) {
    /// caps.http
    ///     .post("https://httpbin.org/json")
    ///     .expect_string()
    ///     .send(Event::ReceiveResponse)
    /// # }
    /// ```
    pub fn expect_string(self) -> RequestBuilder<Event, String> {
        let expectation = Box::<ExpectString>::default();
        RequestBuilder {
            req: self.req,
            cap_or_client: self.cap_or_client,
            phantom: PhantomData,
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
    /// # struct Capabilities { http: crux_http::Http<Event> }
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
    /// # fn update(caps: &Capabilities) {
    /// caps.http
    ///     .post("https://httpbin.org/json")
    ///     .expect_json::<Slideshow>()
    ///     .send(Event::ReceiveResponse)
    /// # }
    /// ```
    pub fn expect_json<T>(self) -> RequestBuilder<Event, T>
    where
        T: DeserializeOwned + 'static,
    {
        let expectation = Box::<ExpectJson<T>>::default();
        RequestBuilder {
            req: self.req,
            cap_or_client: self.cap_or_client,
            phantom: PhantomData,
            expectation,
        }
    }

    /// Sends the constructed `Request` and returns its result as an update `Event`
    ///
    /// When finished, the response will wrapped in an event using `make_event` and
    /// dispatched to the app's `update function.
    pub fn send<F>(self, make_event: F)
    where
        F: FnOnce(crate::Result<Response<ExpectBody>>) -> Event + Send + 'static,
    {
        let CapOrClient::Capability(capability) = self.cap_or_client else {
            panic!("Called RequestBuilder::send in a middleware context");
        };
        let request = self.req;

        let ctx = capability.context.clone();
        ctx.spawn(async move {
            let result = capability.client.send(request.unwrap()).await;

            let resp = match result {
                Ok(resp) => resp,
                Err(e) => {
                    capability.context.update_app(make_event(Err(e)));
                    return;
                }
            };

            let resp = Response::<Vec<u8>>::new(resp)
                .await
                .and_then(|r| self.expectation.decode(r));

            capability.context.update_app(make_event(resp));
        });
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
    pub fn send_async(self) -> BoxFuture<'static, Result<ResponseAsync>> {
        <Self as std::future::IntoFuture>::into_future(self)
    }
}

impl<T, Eb> std::future::IntoFuture for RequestBuilder<T, Eb> {
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
