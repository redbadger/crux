use crate::expect::{ExpectBytes, ExpectJson};
use crate::middleware::Middleware;
use crate::{
    expect::ResponseExpectation,
    http::{
        headers::{HeaderName, ToHeaderValues},
        Body, Method, Mime, Url,
    },
};
use crate::{Client, Error, Request, Response, ResponseAsync, Result};

use futures_util::future::BoxFuture;
use http_types::convert::DeserializeOwned;
use serde::Serialize;

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{fmt, marker::PhantomData};

/// Request Builder
///
/// Provides an ergonomic way to chain the creation of a request.
/// This is generally accessed as the return value from `crux_http::{method}()`,
/// however [`Request::builder`](crate::Request::builder) is also provided.
///
/// # Examples
///
/// ```rust
/// use crux_http::http::{Method, mime::HTML, Url};
/// # #[async_std::main]
/// # async fn main() -> crux_http::Result<()> {
/// let mut request = crux_http::post("https://httpbin.org/post")
///     .body("<html>hi</html>")
///     .header("custom-header", "value")
///     .content_type(HTML)
///     .build();
///
/// assert_eq!(request.take_body().into_string().await.unwrap(), "<html>hi</html>");
/// assert_eq!(request.method(), Method::Post);
/// assert_eq!(request.url(), &Url::parse("https://httpbin.org/post")?);
/// assert_eq!(request["custom-header"], "value");
/// assert_eq!(request["content-type"], "text/html;charset=utf-8");
/// # Ok(())
/// # }
/// ```
///
/// ```rust
/// use crux_http::http::{Method, Url};
/// # #[async_std::main]
/// # async fn main() -> crux_http::Result<()> {
/// let url = Url::parse("https://httpbin.org/post")?;
/// let request = crux_http::Request::builder(Method::Post, url).build();
/// # Ok(())
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
    /// ```
    /// let req = crux_http::get("https://httpbin.org/get").header("header-name", "header-value").build();
    /// assert_eq!(req["header-name"], "header-value");
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
    /// # use crux_http::http::mime;
    /// let req = crux_http::post("https://httpbin.org/post").content_type(mime::HTML).build();
    /// assert_eq!(req["content-type"], "text/html;charset=utf-8");
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
    /// ```
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// use serde_json::json;
    /// let mut req = crux_http::post("https://httpbin.org/post").body(json!({ "any": "Into<Body>"})).build();
    /// assert_eq!(req.take_body().into_string().await.unwrap(), "{\"any\":\"Into<Body>\"}");
    /// # Ok(())
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
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// #[derive(Deserialize, Serialize)]
    /// struct Ip {
    ///     ip: String
    /// }
    ///
    /// let uri = "https://httpbin.org/post";
    /// let data = &Ip { ip: "129.0.0.1".into() };
    /// let res = crux_http::post(uri).body_json(data)?.await?;
    /// assert_eq!(res.status(), 200);
    /// # Ok(()) }
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
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let uri = "https://httpbin.org/post";
    /// let data = "hello world".to_string();
    /// let res = crux_http::post(uri).body_string(data).await?;
    /// assert_eq!(res.status(), 200);
    /// # Ok(()) }
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
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let uri = "https://httpbin.org/post";
    /// let data = b"hello world".to_owned();
    /// let res = crux_http::post(uri).body_bytes(data).await?;
    /// assert_eq!(res.status(), 200);
    /// # Ok(()) }
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
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// #[derive(Serialize, Deserialize)]
    /// struct Index {
    ///     page: u32
    /// }
    ///
    /// let query = Index { page: 2 };
    /// let mut req = crux_http::get("https://httpbin.org/get").query(&query)?.build();
    /// assert_eq!(req.url().query(), Some("page=2"));
    /// assert_eq!(req.url().as_str(), "https://httpbin.org/get?page=2");
    /// # Ok(()) }
    /// ```
    pub fn query(mut self, query: &impl Serialize) -> std::result::Result<Self, Error> {
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
    /// # #[async_std::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    /// let res = crux_http::get("https://httpbin.org/get")
    ///     .middleware(crux_http::middleware::Redirect::default())
    ///     .await?;
    /// # Ok(()) }
    /// ```
    pub fn middleware(mut self, middleware: impl Middleware) -> Self {
        self.req.as_mut().unwrap().middleware(middleware);
        self
    }

    /// Return the constructed `Request`.
    pub fn build(self) -> Request {
        self.req.unwrap()
    }

    // TODO: Ideally this would only be allowed where Event != ()
    pub fn expect_json<T>(self) -> RequestBuilder<Event, T>
    where
        T: DeserializeOwned + 'static,
    {
        let expectation = Box::new(ExpectJson::<T>::default());
        RequestBuilder {
            req: self.req,
            cap_or_client: self.cap_or_client,
            phantom: PhantomData,
            expectation,
        }
    }

    // TODO: Ideally this would only be allowed where Event != ()
    /// Sends the constructed `Request` and returns its result as an update `Event`
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

            // Note: doing an unwrap here, but since we're reading bytes from
            // a prepopulated buffer there should be no way for this to fail
            // currently.
            let resp = Response::<Vec<u8>>::new(resp).await.unwrap();

            capability
                .context
                .update_app(make_event(self.expectation.decode(resp)));
        });
    }
}

impl RequestBuilder<()> {
    /// A send function for middlewares to use that bypasses the usual "we must return an event"
    /// stuff.
    pub async fn middleware_send(self) -> Result<ResponseAsync> {
        let CapOrClient::Client(client) = self.cap_or_client else {
            panic!("Called RequestBuilder::middleware_send in a non-middleware context");
        };

        client.send(self.req.unwrap()).await
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
