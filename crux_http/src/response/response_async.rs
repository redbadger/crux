use http::{HeaderMap, HeaderName, HeaderValue, StatusCode, Version};
use serde::de::DeserializeOwned;
use std::fmt;
use std::ops::Index;

use super::decode::decode_body;
use crate::protocol::HttpResponse;

/// An HTTP response that exposes async methods for use in middleware.
pub struct ResponseAsync {
    status: StatusCode,
    version: Option<Version>,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl ResponseAsync {
    /// Create a new instance directly from parts.
    pub(crate) fn new(status: StatusCode, headers: HeaderMap, body: Vec<u8>) -> Self {
        Self {
            status,
            version: None,
            headers,
            body,
        }
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
    #[must_use]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Get the HTTP protocol version.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crux_http::client::Client;
    /// # async fn middleware(client: Client) -> crux_http::Result<()> {
    /// use crux_http::http::Version;
    /// let res = client.get("https://httpbin.org/get").await?;
    /// assert_eq!(res.version(), Some(Version::HTTP_11));
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn version(&self) -> Option<Version> {
        self.version
    }

    /// Get all values for a header name.
    pub fn header_all(
        &self,
        name: impl http::header::AsHeaderName,
    ) -> http::header::GetAll<'_, HeaderValue> {
        self.headers.get_all(name)
    }

    /// Get a header value by name (returns the first value for that name).
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
    pub fn header(&self, name: impl http::header::AsHeaderName) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

    /// Get a header value mutably.
    pub fn header_mut(
        &mut self,
        name: impl http::header::AsHeaderName,
    ) -> Option<&mut HeaderValue> {
        self.headers.get_mut(name)
    }

    /// Remove a header.
    pub fn remove_header(&mut self, name: impl http::header::AsHeaderName) -> Option<HeaderValue> {
        self.headers.remove(name)
    }

    /// Insert an HTTP header, replacing any existing value.
    pub fn insert_header(
        &mut self,
        key: impl http::header::IntoHeaderName,
        value: impl AsRef<str>,
    ) {
        if let Ok(v) = HeaderValue::from_str(value.as_ref()) {
            self.headers.insert(key, v);
        }
    }

    /// Append an HTTP header, keeping any existing values.
    pub fn append_header(
        &mut self,
        key: impl http::header::IntoHeaderName,
        value: impl AsRef<str>,
    ) {
        if let Ok(v) = HeaderValue::from_str(value.as_ref()) {
            self.headers.append(key, v);
        }
    }

    /// An iterator visiting all header (name, value) pairs in arbitrary order.
    #[must_use]
    pub fn iter(&self) -> http::header::Iter<'_, HeaderValue> {
        self.headers.iter()
    }

    /// An iterator visiting all header (name, value) pairs with mutable values.
    #[must_use]
    pub fn iter_mut(&mut self) -> http::header::IterMut<'_, HeaderValue> {
        self.headers.iter_mut()
    }

    /// An iterator visiting all header names in arbitrary order.
    #[must_use]
    pub fn header_names(&self) -> http::header::Keys<'_, HeaderValue> {
        self.headers.keys()
    }

    /// An iterator visiting all header values in arbitrary order.
    #[must_use]
    pub fn header_values(&self) -> http::header::Values<'_, HeaderValue> {
        self.headers.values()
    }

    /// Get the response content type as a `Mime`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crux_http::client::Client;
    /// # async fn middleware(client: Client) -> crux_http::Result<()> {
    /// let res = client.get("https://httpbin.org/json").await?;
    /// assert_eq!(res.content_type(), Some(mime::APPLICATION_JSON));
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn content_type(&self) -> Option<mime::Mime> {
        self.headers
            .get(http::header::CONTENT_TYPE)?
            .to_str()
            .ok()?
            .parse()
            .ok()
    }

    /// Get the length of the body in bytes.
    #[must_use]
    pub fn len(&self) -> Option<usize> {
        Some(self.body.len())
    }

    /// Returns `true` if the body is empty.
    #[must_use]
    pub fn is_empty(&self) -> Option<bool> {
        Some(self.body.is_empty())
    }

    /// Reads the entire response body into a byte buffer, leaving it empty.
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
        Ok(std::mem::take(&mut self.body))
    }

    /// Reads the entire response body into a string.
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
            .and_then(|m| m.get_param(mime::CHARSET))
            .map(|name| name.as_str().to_owned());
        Ok(decode_body(bytes, claimed_encoding.as_deref())?)
    }

    /// Reads and deserializes the entire response body from JSON.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use serde::{Deserialize, Serialize};
    /// # use crux_http::client::Client;
    /// # async fn middleware(client: Client) -> crux_http::Result<()> {
    /// #[derive(Deserialize, Serialize)]
    /// struct Ip { ip: String }
    /// let mut res = client.get("https://api.ipify.org?format=json").await?;
    /// let Ip { ip } = res.body_json().await?;
    /// # Ok(()) }
    /// ```
    pub async fn body_json<T: DeserializeOwned>(&mut self) -> crate::Result<T> {
        let body_bytes = self.body_bytes().await?;
        serde_json::from_slice(&body_bytes).map_err(crate::HttpError::from)
    }

    /// Reads and deserializes the entire response body from form encoding.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use serde::{Deserialize, Serialize};
    /// # use crux_http::client::Client;
    /// # async fn middleware(client: Client) -> crux_http::Result<()> {
    /// #[derive(Deserialize, Serialize)]
    /// struct Body { apples: u32 }
    /// let mut res = client.get("https://api.example.com/v1/response").await?;
    /// let Body { apples } = res.body_form().await?;
    /// # Ok(()) }
    /// ```
    pub async fn body_form<T: DeserializeOwned>(&mut self) -> crate::Result<T> {
        let bytes = self.body_bytes().await?;
        serde_qs::from_bytes(&bytes).map_err(crate::HttpError::from)
    }
}

impl AsRef<HeaderMap> for ResponseAsync {
    fn as_ref(&self) -> &HeaderMap {
        &self.headers
    }
}

impl AsMut<HeaderMap> for ResponseAsync {
    fn as_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }
}

impl From<HttpResponse> for ResponseAsync {
    fn from(r: HttpResponse) -> Self {
        let mut headers = HeaderMap::new();
        for header in r.headers {
            if let (Ok(name), Ok(value)) = (
                HeaderName::from_bytes(header.name.as_bytes()),
                HeaderValue::from_str(&header.value),
            ) {
                headers.append(name, value);
            }
        }
        Self::new(
            StatusCode::from_u16(r.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            headers,
            r.body,
        )
    }
}

impl<'a> IntoIterator for &'a ResponseAsync {
    type Item = (&'a HeaderName, &'a HeaderValue);
    type IntoIter = http::header::Iter<'a, HeaderValue>;
    fn into_iter(self) -> Self::IntoIter {
        self.headers.iter()
    }
}

impl<'a> IntoIterator for &'a mut ResponseAsync {
    type Item = (&'a HeaderName, &'a mut HeaderValue);
    type IntoIter = http::header::IterMut<'a, HeaderValue>;
    fn into_iter(self) -> Self::IntoIter {
        self.headers.iter_mut()
    }
}

impl fmt::Debug for ResponseAsync {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResponseAsync")
            .field("status", &self.status)
            .field("headers", &self.headers)
            .finish_non_exhaustive()
    }
}

impl Index<&str> for ResponseAsync {
    type Output = HeaderValue;

    /// Returns a reference to the first header value for the given name.
    ///
    /// # Panics
    ///
    /// Panics if the name is not present in `ResponseAsync`.
    #[inline]
    fn index(&self, name: &str) -> &HeaderValue {
        &self.headers[name]
    }
}
