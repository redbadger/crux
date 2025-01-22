use crate::middleware::Middleware;
use http_types::{
    headers::{self, HeaderName, HeaderValues, ToHeaderValues},
    Body, Method, Mime, Url,
};

use serde::Serialize;

use std::fmt;
use std::ops::Index;
use std::sync::Arc;

/// An HTTP request, returns a `Response`.
#[derive(Clone)]
pub struct Request {
    /// Holds the state of the request.
    req: http_types::Request,
    /// Holds an optional per-request middleware stack.
    middleware: Option<Vec<Arc<dyn Middleware>>>,
}

impl Request {
    /// Create a new instance.
    ///
    /// This method is particularly useful when input URLs might be passed by third parties, and
    /// you don't want to panic if they're malformed. If URLs are statically encoded, it might be
    /// easier to use one of the shorthand methods instead.
    ///
    /// # Examples
    ///
    /// ```
    /// fn main() -> crux_http::Result<()> {
    /// use crux_http::http::{Url, Method};
    ///
    /// let url = Url::parse("https://httpbin.org/get")?;
    /// let req = crux_http::Request::new(Method::Get, url);
    /// # Ok(()) }
    /// ```
    pub fn new(method: Method, url: Url) -> Self {
        let req = http_types::Request::new(method, url);
        Self {
            req,
            middleware: None,
        }
    }

    /// Get the URL querystring.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde::{Deserialize, Serialize};
    /// # enum Event {}
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) -> crux_http::Result<()> {
    /// #[derive(Serialize, Deserialize)]
    /// struct Index {
    ///     page: u32
    /// }
    ///
    /// let req = caps.http.get("https://httpbin.org/get?page=2").build();
    /// let Index { page } = req.query()?;
    /// assert_eq!(page, 2);
    /// # Ok(()) }
    /// ```
    pub fn query<T: serde::de::DeserializeOwned>(&self) -> crate::Result<T> {
        Ok(self.req.query()?)
    }

    /// Set the URL querystring.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde::{Deserialize, Serialize};
    /// # enum Event {}
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) -> crux_http::Result<()> {
    /// #[derive(Serialize, Deserialize)]
    /// struct Index {
    ///     page: u32
    /// }
    ///
    /// let query = Index { page: 2 };
    /// let mut req = caps.http.get("https://httpbin.org/get").build();
    /// req.set_query(&query)?;
    /// assert_eq!(req.url().query(), Some("page=2"));
    /// assert_eq!(req.url().as_str(), "https://httpbin.org/get?page=2");
    /// # Ok(()) }
    /// ```
    pub fn set_query(&mut self, query: &impl Serialize) -> crate::Result<()> {
        Ok(self.req.set_query(query)?)
    }

    /// Get an HTTP header.
    ///
    /// # Examples
    ///
    /// ```
    /// # enum Event {}
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) -> crux_http::Result<()> {
    /// let mut req = caps.http.get("https://httpbin.org/get").build();
    /// req.set_header("X-Requested-With", "surf");
    /// assert_eq!(req.header("X-Requested-With").unwrap(), "surf");
    /// # Ok(()) }
    /// ```
    pub fn header(&self, key: impl Into<HeaderName>) -> Option<&HeaderValues> {
        self.req.header(key)
    }

    /// Get a mutable reference to a header.
    pub fn header_mut(&mut self, name: impl Into<HeaderName>) -> Option<&mut HeaderValues> {
        self.req.header_mut(name)
    }

    /// Set an HTTP header.
    pub fn insert_header(
        &mut self,
        name: impl Into<HeaderName>,
        values: impl ToHeaderValues,
    ) -> Option<HeaderValues> {
        self.req.insert_header(name, values)
    }

    /// Append a header to the headers.
    ///
    /// Unlike `insert` this function will not override the contents of a header, but insert a
    /// header if there aren't any. Or else append to the existing list of headers.
    pub fn append_header(&mut self, name: impl Into<HeaderName>, values: impl ToHeaderValues) {
        self.req.append_header(name, values)
    }

    /// Remove a header.
    pub fn remove_header(&mut self, name: impl Into<HeaderName>) -> Option<HeaderValues> {
        self.req.remove_header(name)
    }

    /// An iterator visiting all header pairs in arbitrary order.
    #[must_use]
    pub fn iter(&self) -> headers::Iter<'_> {
        self.req.iter()
    }

    /// An iterator visiting all header pairs in arbitrary order, with mutable references to the
    /// values.
    #[must_use]
    pub fn iter_mut(&mut self) -> headers::IterMut<'_> {
        self.req.iter_mut()
    }

    /// An iterator visiting all header names in arbitrary order.
    #[must_use]
    pub fn header_names(&self) -> headers::Names<'_> {
        self.req.header_names()
    }

    /// An iterator visiting all header values in arbitrary order.
    #[must_use]
    pub fn header_values(&self) -> headers::Values<'_> {
        self.req.header_values()
    }

    /// Set an HTTP header.
    ///
    /// # Examples
    ///
    /// ```
    /// # enum Event {}
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) -> crux_http::Result<()> {
    /// let mut req = caps.http.get("https://httpbin.org/get").build();
    /// req.set_header("X-Requested-With", "surf");
    /// assert_eq!(req.header("X-Requested-With").unwrap(), "surf");
    /// # Ok(()) }
    /// ```
    pub fn set_header(&mut self, key: impl Into<HeaderName>, value: impl ToHeaderValues) {
        self.insert_header(key, value);
    }

    /// Get a request extension value.
    #[must_use]
    pub fn ext<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.req.ext().get()
    }

    /// Set a request extension value.
    pub fn set_ext<T: Send + Sync + 'static>(&mut self, val: T) -> Option<T> {
        self.req.ext_mut().insert(val)
    }

    /// Get the request HTTP method.
    ///
    /// # Examples
    ///
    /// ```
    /// # enum Event {}
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) -> crux_http::Result<()> {
    /// let req = caps.http.get("https://httpbin.org/get").build();
    /// assert_eq!(req.method(), crux_http::http::Method::Get);
    /// # Ok(()) }
    /// ```
    pub fn method(&self) -> Method {
        self.req.method()
    }

    /// Get the request url.
    ///
    /// # Examples
    ///
    /// ```
    /// # enum Event {}
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) -> crux_http::Result<()> {
    /// use crux_http::http::Url;
    /// let req = caps.http.get("https://httpbin.org/get").build();
    /// assert_eq!(req.url(), &Url::parse("https://httpbin.org/get")?);
    /// # Ok(()) }
    /// ```
    pub fn url(&self) -> &Url {
        self.req.url()
    }

    /// Get the request content type as a `Mime`.
    ///
    /// Gets the `Content-Type` header and parses it to a `Mime` type.
    ///
    /// [Read more on MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types)
    ///
    /// # Panics
    ///
    /// This method will panic if an invalid MIME type was set as a header. Use the [`set_header`]
    /// method to bypass any checks.
    ///
    /// [`set_header`]: #method.set_header
    pub fn content_type(&self) -> Option<Mime> {
        self.req.content_type()
    }

    /// Set the request content type from a `Mime`.
    ///
    /// [Read more on MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types)
    pub fn set_content_type(&mut self, mime: Mime) {
        self.req.set_content_type(mime);
    }

    /// Get the length of the body stream, if it has been set.
    ///
    /// This value is set when passing a fixed-size object into as the body.
    /// E.g. a string, or a buffer. Consumers of this API should check this
    /// value to decide whether to use `Chunked` encoding, or set the
    /// response length.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> Option<usize> {
        self.req.len()
    }

    /// Returns `true` if the set length of the body stream is zero, `false`
    /// otherwise.
    pub fn is_empty(&self) -> Option<bool> {
        self.req.is_empty()
    }

    /// Pass an `AsyncRead` stream as the request body.
    ///
    /// # Mime
    ///
    /// The encoding is set to `application/octet-stream`.
    pub fn set_body(&mut self, body: impl Into<Body>) {
        self.req.set_body(body)
    }

    /// Take the request body as a `Body`.
    ///
    /// This method can be called after the body has already been taken or read,
    /// but will return an empty `Body`.
    ///
    /// This is useful for consuming the body via an AsyncReader or AsyncBufReader.
    pub fn take_body(&mut self) -> Body {
        self.req.take_body()
    }

    /// Pass JSON as the request body.
    ///
    /// # Mime
    ///
    /// The `content-type` is set to `application/json`.
    ///
    /// # Errors
    ///
    /// This method will return an error if the provided data could not be serialized to JSON.
    pub fn body_json(&mut self, json: &impl Serialize) -> crate::Result<()> {
        self.set_body(Body::from_json(json)?);
        Ok(())
    }

    /// Pass a string as the request body.
    ///
    /// # Mime
    ///
    /// The `content-type` is set to `text/plain; charset=utf-8`.
    pub fn body_string(&mut self, string: String) {
        self.set_body(Body::from_string(string))
    }

    /// Pass bytes as the request body.
    ///
    /// # Mime
    ///
    /// The `content-type` is set to `application/octet-stream`.
    pub fn body_bytes(&mut self, bytes: impl AsRef<[u8]>) {
        self.set_body(Body::from(bytes.as_ref()))
    }

    /// Pass a form as the request body.
    ///
    /// # Mime
    ///
    /// The `content-type` is set to `application/x-www-form-urlencoded`.
    ///
    /// # Errors
    ///
    /// An error will be returned if the encoding failed.
    pub fn body_form(&mut self, form: &impl Serialize) -> crate::Result<()> {
        self.set_body(Body::from_form(form)?);
        Ok(())
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
    /// # enum Event {}
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) -> crux_http::Result<()> {
    /// let mut req = caps.http.get("https://httpbin.org/get").build();
    /// req.middleware(crux_http::middleware::Redirect::default());
    /// # Ok(()) }
    /// ```
    pub fn middleware(&mut self, middleware: impl Middleware) {
        if self.middleware.is_none() {
            self.middleware = Some(vec![]);
        }

        self.middleware.as_mut().unwrap().push(Arc::new(middleware));
    }

    pub(crate) fn take_middleware(&mut self) -> Option<Vec<Arc<dyn Middleware>>> {
        self.middleware.take()
    }
}

impl AsRef<http_types::Headers> for Request {
    fn as_ref(&self) -> &http_types::Headers {
        self.req.as_ref()
    }
}

impl AsMut<http_types::Headers> for Request {
    fn as_mut(&mut self) -> &mut http_types::Headers {
        self.req.as_mut()
    }
}

impl AsRef<http_types::Request> for Request {
    fn as_ref(&self) -> &http_types::Request {
        &self.req
    }
}

impl AsMut<http_types::Request> for Request {
    fn as_mut(&mut self) -> &mut http_types::Request {
        &mut self.req
    }
}

impl From<http_types::Request> for Request {
    /// Converts an `http_types::Request` to a `crux_http::Request`.
    fn from(req: http_types::Request) -> Self {
        Self {
            req,
            middleware: None,
        }
    }
}

#[cfg(feature = "http-compat")]
impl<B: Into<Body>> TryFrom<http::Request<B>> for Request {
    type Error = anyhow::Error;

    fn try_from(req: http::Request<B>) -> Result<Self, Self::Error> {
        use std::str::FromStr;
        let mut o = Request::new(
            Method::from_str(req.method().as_str()).map_err(|e| anyhow::anyhow!(e))?,
            req.uri().to_string().parse()?,
        );

        for (k, v) in req.headers().iter() {
            o.append_header(k.as_str(), v.to_str()?);
        }

        o.set_body(req.into_body());
        Ok(o)
    }
}

#[allow(clippy::from_over_into)]
impl Into<http_types::Request> for Request {
    /// Converts a `crux_http::Request` to an `http_types::Request`.
    fn into(self) -> http_types::Request {
        self.req
    }
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.req, f)
    }
}

impl IntoIterator for Request {
    type Item = (HeaderName, HeaderValues);
    type IntoIter = headers::IntoIter;

    /// Returns an iterator of references over the remaining items.
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.req.into_iter()
    }
}

impl<'a> IntoIterator for &'a Request {
    type Item = (&'a HeaderName, &'a HeaderValues);
    type IntoIter = headers::Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.req.iter()
    }
}

impl<'a> IntoIterator for &'a mut Request {
    type Item = (&'a HeaderName, &'a mut HeaderValues);
    type IntoIter = headers::IterMut<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.req.iter_mut()
    }
}

impl Index<HeaderName> for Request {
    type Output = HeaderValues;

    /// Returns a reference to the value corresponding to the supplied name.
    ///
    /// # Panics
    ///
    /// Panics if the name is not present in `Request`.
    #[inline]
    fn index(&self, name: HeaderName) -> &HeaderValues {
        &self.req[name]
    }
}

impl Index<&str> for Request {
    type Output = HeaderValues;

    /// Returns a reference to the value corresponding to the supplied name.
    ///
    /// # Panics
    ///
    /// Panics if the name is not present in `Request`.
    #[inline]
    fn index(&self, name: &str) -> &HeaderValues {
        &self.req[name]
    }
}
