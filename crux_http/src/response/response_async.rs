use http_types::{
    self,
    headers::{self, HeaderName, HeaderValues, ToHeaderValues},
    Body, Mime, StatusCode, Version,
};

use futures_util::io::AsyncRead;
use serde::de::DeserializeOwned;

use std::fmt;
use std::io;
use std::ops::Index;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::decode::decode_body;

pin_project_lite::pin_project! {
    /// An HTTP response that exposes async methods. This is to support async
    /// use and middleware.
    pub struct ResponseAsync {
        #[pin]
        res: http_types::Response,
    }
}

impl ResponseAsync {
    /// Create a new instance.
    pub(crate) fn new(res: http_types::Response) -> Self {
        Self { res }
    }

    /// Get the HTTP status code.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crux_http::client::Client;
    /// # async fn middleware(client: Client) -> crux_http::Result<()> {
    /// let res = client.get("https://httpbin.org/get").await?;
    /// assert_eq!(res.status(), 200);
    /// # Ok(()) }
    /// ```
    pub fn status(&self) -> StatusCode {
        self.res.status()
    }

    /// Get the HTTP protocol version.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crux_http::client::Client;
    /// # async fn middleware(client: Client) -> crux_http::Result<()> {
    /// use crux_http::http::Version;
    ///
    /// let res = client.get("https://httpbin.org/get").await?;
    /// assert_eq!(res.version(), Some(Version::Http1_1));
    /// # Ok(()) }
    /// ```
    pub fn version(&self) -> Option<Version> {
        self.res.version()
    }

    /// Get a header.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crux_http::client::Client;
    /// # async fn middleware(client: Client) -> crux_http::Result<()> {
    /// let res = client.get("https://httpbin.org/get").await?;
    /// assert!(res.header("Content-Length").is_some());
    /// # Ok(()) }
    /// ```
    pub fn header(&self, name: impl Into<HeaderName>) -> Option<&HeaderValues> {
        self.res.header(name)
    }

    /// Get an HTTP header mutably.
    pub fn header_mut(&mut self, name: impl Into<HeaderName>) -> Option<&mut HeaderValues> {
        self.res.header_mut(name)
    }

    /// Remove a header.
    pub fn remove_header(&mut self, name: impl Into<HeaderName>) -> Option<HeaderValues> {
        self.res.remove_header(name)
    }

    /// Insert an HTTP header.
    pub fn insert_header(&mut self, key: impl Into<HeaderName>, value: impl ToHeaderValues) {
        self.res.insert_header(key, value);
    }

    /// Append an HTTP header.
    pub fn append_header(&mut self, key: impl Into<HeaderName>, value: impl ToHeaderValues) {
        self.res.append_header(key, value);
    }

    /// An iterator visiting all header pairs in arbitrary order.
    #[must_use]
    pub fn iter(&self) -> headers::Iter<'_> {
        self.res.iter()
    }

    /// An iterator visiting all header pairs in arbitrary order, with mutable references to the
    /// values.
    #[must_use]
    pub fn iter_mut(&mut self) -> headers::IterMut<'_> {
        self.res.iter_mut()
    }

    /// An iterator visiting all header names in arbitrary order.
    #[must_use]
    pub fn header_names(&self) -> headers::Names<'_> {
        self.res.header_names()
    }

    /// An iterator visiting all header values in arbitrary order.
    #[must_use]
    pub fn header_values(&self) -> headers::Values<'_> {
        self.res.header_values()
    }

    /// Get a response scoped extension value.
    #[must_use]
    pub fn ext<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.res.ext().get()
    }

    /// Set a response scoped extension value.
    pub fn insert_ext<T: Send + Sync + 'static>(&mut self, val: T) {
        self.res.ext_mut().insert(val);
    }

    /// Get the response content type as a `Mime`.
    ///
    /// Gets the `Content-Type` header and parses it to a `Mime` type.
    ///
    /// [Read more on MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types)
    ///
    /// # Panics
    ///
    /// This method will panic if an invalid MIME type was set as a header.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crux_http::client::Client;
    /// # async fn middleware(client: Client) -> crux_http::Result<()> {
    /// use crux_http::http::mime;
    /// let res = client.get("https://httpbin.org/json").await?;
    /// assert_eq!(res.content_type(), Some(mime::JSON));
    /// # Ok(()) }
    /// ```
    pub fn content_type(&self) -> Option<Mime> {
        self.res.content_type()
    }

    /// Get the length of the body stream, if it has been set.
    ///
    /// This value is set when passing a fixed-size object into as the body.
    /// E.g. a string, or a buffer. Consumers of this API should check this
    /// value to decide whether to use `Chunked` encoding, or set the
    /// response length.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> Option<usize> {
        self.res.len()
    }

    /// Returns `true` if the set length of the body stream is zero, `false`
    /// otherwise.
    pub fn is_empty(&self) -> Option<bool> {
        self.res.is_empty()
    }

    /// Set the body reader.
    pub fn set_body(&mut self, body: impl Into<Body>) {
        self.res.set_body(body);
    }

    /// Take the response body as a `Body`.
    ///
    /// This method can be called after the body has already been taken or read,
    /// but will return an empty `Body`.
    ///
    /// Useful for adjusting the whole body, such as in middleware.
    pub fn take_body(&mut self) -> Body {
        self.res.take_body()
    }

    /// Swaps the value of the body with another body, without deinitializing
    /// either one.
    pub fn swap_body(&mut self, body: &mut Body) {
        self.res.swap_body(body)
    }

    /// Reads the entire request body into a byte buffer.
    ///
    /// This method can be called after the body has already been read, but will
    /// produce an empty buffer.
    ///
    /// # Errors
    ///
    /// Any I/O error encountered while reading the body is immediately returned
    /// as an `Err`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crux_http::client::Client;
    /// # async fn middleware(client: Client) -> crux_http::Result<()> {
    /// let mut res = client.get("https://httpbin.org/get").await?;
    /// let bytes: Vec<u8> = res.body_bytes().await?;
    /// # Ok(()) }
    /// ```
    pub async fn body_bytes(&mut self) -> crate::Result<Vec<u8>> {
        Ok(self.res.body_bytes().await?)
    }

    /// Reads the entire response body into a string.
    ///
    /// This method can be called after the body has already been read, but will
    /// produce an empty buffer.
    ///
    /// # Encodings
    ///
    /// If the "encoding" feature is enabled, this method tries to decode the body
    /// with the encoding that is specified in the Content-Type header. If the header
    /// does not specify an encoding, UTF-8 is assumed. If the "encoding" feature is
    /// disabled, Surf only supports reading UTF-8 response bodies. The "encoding"
    /// feature is enabled by default.
    ///
    /// # Errors
    ///
    /// Any I/O error encountered while reading the body is immediately returned
    /// as an `Err`.
    ///
    /// If the body cannot be interpreted because the encoding is unsupported or
    /// incorrect, an `Err` is returned.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crux_http::client::Client;
    /// # async fn middleware(client: Client) -> crux_http::Result<()> {
    /// let mut res = client.get("https://httpbin.org/get").await?;
    /// let string: String = res.body_string().await?;
    /// # Ok(()) }
    /// ```
    pub async fn body_string(&mut self) -> crate::Result<String> {
        let bytes = self.body_bytes().await?;
        let mime = self.content_type();
        let claimed_encoding = mime
            .as_ref()
            .and_then(|mime| mime.param("charset"))
            .map(|name| name.to_string());
        Ok(decode_body(bytes, claimed_encoding.as_deref())?)
    }

    /// Reads and deserialized the entire request body from json.
    ///
    /// # Errors
    ///
    /// Any I/O error encountered while reading the body is immediately returned
    /// as an `Err`.
    ///
    /// If the body cannot be interpreted as valid json for the target type `T`,
    /// an `Err` is returned.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use serde::{Deserialize, Serialize};
    /// # use crux_http::client::Client;
    /// # async fn middleware(client: Client) -> crux_http::Result<()> {
    /// #[derive(Deserialize, Serialize)]
    /// struct Ip {
    ///     ip: String
    /// }
    ///
    /// let mut res = client.get("https://api.ipify.org?format=json").await?;
    /// let Ip { ip } = res.body_json().await?;
    /// # Ok(()) }
    /// ```
    pub async fn body_json<T: DeserializeOwned>(&mut self) -> crate::Result<T> {
        let body_bytes = self.body_bytes().await?;
        serde_json::from_slice(&body_bytes).map_err(crate::HttpError::from)
    }

    /// Reads and deserialized the entire request body from form encoding.
    ///
    /// # Errors
    ///
    /// Any I/O error encountered while reading the body is immediately returned
    /// as an `Err`.
    ///
    /// If the body cannot be interpreted as valid json for the target type `T`,
    /// an `Err` is returned.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use serde::{Deserialize, Serialize};
    /// # use crux_http::client::Client;
    /// # async fn middleware(client: Client) -> crux_http::Result<()> {
    /// #[derive(Deserialize, Serialize)]
    /// struct Body {
    ///     apples: u32
    /// }
    ///
    /// let mut res = client.get("https://api.example.com/v1/response").await?;
    /// let Body { apples } = res.body_form().await?;
    /// # Ok(()) }
    /// ```
    pub async fn body_form<T: serde::de::DeserializeOwned>(&mut self) -> crate::Result<T> {
        Ok(self.res.body_form().await?)
    }
}

impl From<http_types::Response> for ResponseAsync {
    fn from(response: http_types::Response) -> Self {
        Self::new(response)
    }
}

#[allow(clippy::from_over_into)]
impl Into<http_types::Response> for ResponseAsync {
    fn into(self) -> http_types::Response {
        self.res
    }
}

impl AsRef<http_types::Headers> for ResponseAsync {
    fn as_ref(&self) -> &http_types::Headers {
        self.res.as_ref()
    }
}

impl AsMut<http_types::Headers> for ResponseAsync {
    fn as_mut(&mut self) -> &mut http_types::Headers {
        self.res.as_mut()
    }
}

impl AsRef<http_types::Response> for ResponseAsync {
    fn as_ref(&self) -> &http_types::Response {
        &self.res
    }
}

impl AsMut<http_types::Response> for ResponseAsync {
    fn as_mut(&mut self) -> &mut http_types::Response {
        &mut self.res
    }
}

impl AsyncRead for ResponseAsync {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, io::Error>> {
        Pin::new(&mut self.res).poll_read(cx, buf)
    }
}

impl fmt::Debug for ResponseAsync {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("response", &self.res)
            .finish()
    }
}

impl Index<HeaderName> for ResponseAsync {
    type Output = HeaderValues;

    /// Returns a reference to the value corresponding to the supplied name.
    ///
    /// # Panics
    ///
    /// Panics if the name is not present in `Response`.
    #[inline]
    fn index(&self, name: HeaderName) -> &HeaderValues {
        &self.res[name]
    }
}

impl Index<&str> for ResponseAsync {
    type Output = HeaderValues;

    /// Returns a reference to the value corresponding to the supplied name.
    ///
    /// # Panics
    ///
    /// Panics if the name is not present in `Response`.
    #[inline]
    fn index(&self, name: &str) -> &HeaderValues {
        &self.res[name]
    }
}
