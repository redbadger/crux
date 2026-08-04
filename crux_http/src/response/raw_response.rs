use http::{HeaderMap, HeaderName, HeaderValue, StatusCode, Version};
use serde::de::DeserializeOwned;
use std::{fmt, ops::Index};

use super::decode::decode_body;
use crate::{HttpError, Result, protocol::HttpResponse};

/// An in-memory HTTP response as returned by the shell, used in the middleware chain.
pub struct RawResponse {
    status: StatusCode,
    version: Option<Version>,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl RawResponse {
    /// Create a new instance directly from parts.
    pub(crate) const fn new(status: StatusCode, headers: HeaderMap, body: Vec<u8>) -> Self {
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
    #[allow(clippy::missing_const_for_fn)]
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
    #[allow(clippy::missing_const_for_fn)]
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
    ///
    /// Returns the previous value for that header name, if any.
    pub fn insert_header(
        &mut self,
        name: impl http::header::IntoHeaderName,
        value: HeaderValue,
    ) -> Option<HeaderValue> {
        self.headers.insert(name, value)
    }

    /// Append an HTTP header, keeping any existing values.
    ///
    /// Returns `true` if the value was appended to an existing entry, `false` if it was the first
    /// value for that name.
    pub fn append_header(
        &mut self,
        name: impl http::header::IntoHeaderName,
        value: HeaderValue,
    ) -> bool {
        self.headers.append(name, value)
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
    #[allow(clippy::missing_const_for_fn)]
    pub fn len(&self) -> Option<usize> {
        Some(self.body.len())
    }

    /// Returns `true` if the body is empty.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn is_empty(&self) -> Option<bool> {
        Some(self.body.is_empty())
    }

    /// Reads the entire response body into a byte buffer, leaving it empty.
    ///
    /// # Errors
    ///
    /// This function currently never returns an error; the `Result` wrapper is kept for API consistency.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crux_http::client::Client;
    /// # async fn middleware(client: Client) -> crux_http::Result<()> {
    /// let mut res = client.get("https://httpbin.org/get").await?;
    /// let bytes: Vec<u8> = res.body_bytes()?;
    /// # Ok(()) }
    /// ```
    pub fn body_bytes(&mut self) -> Result<Vec<u8>> {
        Ok(std::mem::take(&mut self.body))
    }

    /// Reads the entire response body into a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the body contains invalid UTF-8.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crux_http::client::Client;
    /// # async fn middleware(client: Client) -> crux_http::Result<()> {
    /// let mut res = client.get("https://httpbin.org/get").await?;
    /// let string: String = res.body_string()?;
    /// # Ok(()) }
    /// ```
    pub fn body_string(&mut self) -> Result<String> {
        let bytes = self.body_bytes()?;
        let mime = self.content_type();
        let claimed_encoding = mime
            .as_ref()
            .and_then(|m| m.get_param(mime::CHARSET))
            .map(|name| name.as_str().to_owned());
        Ok(decode_body(bytes, claimed_encoding.as_deref())?)
    }

    /// Reads and deserializes the entire response body from JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialisation fails.
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
    /// let Ip { ip } = res.body_json()?;
    /// # Ok(()) }
    /// ```
    pub fn body_json<T: DeserializeOwned>(&mut self) -> Result<T> {
        let body_bytes = self.body_bytes()?;
        serde_json::from_slice(&body_bytes).map_err(HttpError::from)
    }

    /// Reads and deserializes the entire response body from form encoding.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialisation fails.
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
    /// let Body { apples } = res.body_form()?;
    /// # Ok(()) }
    /// ```
    pub fn body_form<T: DeserializeOwned>(&mut self) -> Result<T> {
        let bytes = self.body_bytes()?;
        serde_qs::from_bytes(&bytes).map_err(HttpError::from)
    }
}

impl AsRef<HeaderMap> for RawResponse {
    fn as_ref(&self) -> &HeaderMap {
        &self.headers
    }
}

impl AsMut<HeaderMap> for RawResponse {
    fn as_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }
}

impl TryFrom<HttpResponse> for RawResponse {
    type Error = HttpError;

    fn try_from(r: HttpResponse) -> Result<Self> {
        let HttpResponse {
            status: status_u16,
            headers: header_fields,
            body,
        } = r;

        let status = StatusCode::from_u16(status_u16)
            .map_err(|_| HttpError::InvalidStatusCode(status_u16))?;

        let mut headers = HeaderMap::new();
        for header in header_fields {
            if let (Ok(name), Ok(value)) = (
                HeaderName::from_bytes(header.name.as_bytes()),
                HeaderValue::from_str(&header.value),
            ) {
                headers.append(name, value);
            }
        }
        Ok(Self::new(status, headers, body))
    }
}

impl<'a> IntoIterator for &'a RawResponse {
    type Item = (&'a HeaderName, &'a HeaderValue);
    type IntoIter = http::header::Iter<'a, HeaderValue>;
    fn into_iter(self) -> Self::IntoIter {
        self.headers.iter()
    }
}

impl<'a> IntoIterator for &'a mut RawResponse {
    type Item = (&'a HeaderName, &'a mut HeaderValue);
    type IntoIter = http::header::IterMut<'a, HeaderValue>;
    fn into_iter(self) -> Self::IntoIter {
        self.headers.iter_mut()
    }
}

impl fmt::Debug for RawResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawResponse")
            .field("status", &self.status)
            .field("headers", &self.headers)
            .finish_non_exhaustive()
    }
}

impl Index<&str> for RawResponse {
    type Output = HeaderValue;

    /// Returns a reference to the first header value for the given name.
    ///
    /// # Panics
    ///
    /// Panics if the name is not present in `RawResponse`.
    #[inline]
    fn index(&self, name: &str) -> &HeaderValue {
        &self.headers[name]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::HttpResponse;

    #[test]
    fn from_http_response_preserves_non_standard_status_499() {
        let http_response = HttpResponse::status(499)
            .body(b"client closed connection".to_vec())
            .build();
        let raw = RawResponse::try_from(http_response).expect("499 is a valid status code");
        assert_eq!(raw.status().as_u16(), 499);
        assert!(raw.status().is_client_error());
    }

    #[test]
    fn from_http_response_preserves_non_standard_status_103() {
        // 103 Early Hints (StatusCode::EARLY_HINTS) - verify informational codes are
        // preserved and not mistakenly treated as errors.
        let http_response = HttpResponse::status(103).body(b"".to_vec()).build();
        let raw = RawResponse::try_from(http_response).expect("103 is a valid status code");
        assert_eq!(raw.status().as_u16(), 103);
    }

    #[test]
    fn from_http_response_preserves_edge_case_status_599() {
        let http_response = HttpResponse::status(599)
            .body(b"custom server error".to_vec())
            .build();
        let raw = RawResponse::try_from(http_response).expect("599 is a valid status code");
        assert_eq!(raw.status().as_u16(), 599);
        assert!(raw.status().is_server_error());
    }

    #[test]
    fn from_http_response_rejects_out_of_range_status() {
        for bad_code in [0u16, 99, 1000, u16::MAX] {
            let http_response = HttpResponse::status(bad_code).body(b"".to_vec()).build();
            let err = RawResponse::try_from(http_response)
                .expect_err(&format!("{bad_code} should be rejected"));
            assert!(
                matches!(err, HttpError::InvalidStatusCode(code) if code == bad_code),
                "expected HttpError::InvalidStatusCode({bad_code}), got: {err:?}"
            );
        }
    }
}
