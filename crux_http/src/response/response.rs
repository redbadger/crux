use super::decode::decode_body;
use crate::http::{
    self,
    headers::{self, HeaderName, HeaderValues, ToHeaderValues},
    Error, Mime, StatusCode, Version,
};

use futures_util::io::AsyncRead;
use serde::de::DeserializeOwned;

use std::fmt;
use std::io;
use std::ops::Index;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct Response<Body> {
    res: super::ResponseAsync,
    body: Option<Body>,
}

impl<Body> Response<Body> {
    /// Create a new instance.
    pub(crate) async fn new(res: super::ResponseAsync) -> Self {
        todo!()
        // Self { res }
    }

    /// Get the HTTP status code.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let res = crux_http::get("https://httpbin.org/get").await?;
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
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// use crux_http::http::Version;
    ///
    /// let res = crux_http::get("https://httpbin.org/get").await?;
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
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let res = crux_http::get("https://httpbin.org/get").await?;
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
        self.res.ext::<T>()
    }

    /// Set a response scoped extension value.
    pub fn insert_ext<T: Send + Sync + 'static>(&mut self, val: T) {
        self.res.insert_ext(val)
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
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// use crux_http::http::mime;
    /// let res = crux_http::get("https://httpbin.org/json").await?;
    /// assert_eq!(res.content_type(), Some(mime::JSON));
    /// # Ok(()) }
    /// ```
    pub fn content_type(&self) -> Option<Mime> {
        self.res.content_type()
    }

    pub fn with_body<NewBody>(self, body: NewBody) -> Response<NewBody> {
        Response {
            res: self.res,
            body: Some(body),
        }
    }
}

impl Response<Vec<u8>> {
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
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let mut res = crux_http::get("https://httpbin.org/get").await?;
    /// let bytes: Vec<u8> = res.body_bytes().await?;
    /// # Ok(()) }
    /// ```
    pub fn body_bytes(&mut self) -> crate::Result<Vec<u8>> {
        self.body
            .take()
            .ok_or(crate::Error::from_str(self.status(), "Body had no bytes"))
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
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let mut res = crux_http::get("https://httpbin.org/get").await?;
    /// let string: String = res.body_string().await?;
    /// # Ok(()) }
    /// ```
    pub async fn body_string(&mut self) -> crate::Result<String> {
        let bytes = self.body_bytes()?;

        let mime = self.content_type();
        let claimed_encoding = mime
            .as_ref()
            .and_then(|mime| mime.param("charset"))
            .map(|name| name.to_string());
        decode_body(bytes, claimed_encoding.as_deref())
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
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// #[derive(Deserialize, Serialize)]
    /// struct Ip {
    ///     ip: String
    /// }
    ///
    /// let mut res = crux_http::get("https://api.ipify.org?format=json").await?;
    /// let Ip { ip } = res.body_json().await?;
    /// # Ok(()) }
    /// ```
    pub fn body_json<T: DeserializeOwned>(&mut self) -> crate::Result<T> {
        let body_bytes = self.body_bytes()?;
        serde_json::from_slice(&body_bytes).map_err(crate::Error::from)
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
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// #[derive(Deserialize, Serialize)]
    /// struct Body {
    ///     apples: u32
    /// }
    ///
    /// let mut res = crux_http::get("https://api.example.com/v1/response").await?;
    /// let Body { apples } = res.body_form().await?;
    /// # Ok(()) }
    /// ```
    pub async fn body_form<T: serde::de::DeserializeOwned>(&mut self) -> crate::Result<T> {
        self.res.body_form().await
    }
}

impl<Body> AsRef<http::Headers> for Response<Body> {
    fn as_ref(&self) -> &http::Headers {
        self.res.as_ref()
    }
}

impl<Body> AsMut<http::Headers> for Response<Body> {
    fn as_mut(&mut self) -> &mut http::Headers {
        self.res.as_mut()
    }
}

impl<Body> fmt::Debug for Response<Body> {
    #[allow(missing_doc_code_examples)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("response", &self.res)
            .finish()
    }
}

impl<Body> Index<HeaderName> for Response<Body> {
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

impl<Body> Index<&str> for Response<Body> {
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
