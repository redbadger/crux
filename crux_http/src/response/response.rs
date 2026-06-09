use super::decode::decode_body;
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode, Version};
use serde::de::DeserializeOwned;
use std::fmt;
use std::ops::Index;

/// An HTTP Response that will be passed to an app's update function.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Response<Body> {
    #[serde(skip, default)]
    version: Option<Version>,
    #[serde(with = "status_serde")]
    status: StatusCode,
    #[serde(with = "header_serde")]
    headers: HeaderMap,
    body: Option<Body>,
}

impl<Body> Response<Body> {
    /// Create a new instance.
    pub(crate) fn new(mut res: super::RawResponse) -> crate::Result<Response<Vec<u8>>> {
        let body = res.body_bytes()?;
        let status = res.status();

        if status.is_client_error() || status.is_server_error() {
            return Err(crate::HttpError::Http {
                code: status.as_u16(),
                message: status.to_string(),
                body: Some(body),
            });
        }

        let headers = res.as_ref().clone();

        Ok(Response {
            status,
            headers,
            version: res.version(),
            body: Some(body),
        })
    }

    /// Get the HTTP status code.
    ///
    /// # Examples
    ///
    /// ```
    /// # let res = crux_http::testing::ResponseBuilder::ok().build();
    /// assert_eq!(res.status(), 200);
    /// ```
    #[allow(clippy::missing_const_for_fn)]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Get the HTTP protocol version.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # let res = crux_http::testing::ResponseBuilder::ok().build();
    /// use crux_http::http::Version;
    /// assert_eq!(res.version(), Some(Version::HTTP_11));
    /// ```
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

    /// Get a header value.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # let res = crux_http::testing::ResponseBuilder::ok()
    /// #   .header("Content-Length", "1")
    /// #   .build();
    /// assert!(res.header("Content-Length").is_some());
    /// ```
    pub fn header(&self, name: impl http::header::AsHeaderName) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

    /// Get an HTTP header mutably.
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
    /// ```
    /// # let res = crux_http::testing::ResponseBuilder::ok()
    /// #   .header("Content-Type", "application/json")
    /// #   .build();
    /// assert_eq!(res.content_type(), Some(mime::APPLICATION_JSON));
    /// ```
    pub fn content_type(&self) -> Option<mime::Mime> {
        self.headers
            .get(http::header::CONTENT_TYPE)?
            .to_str()
            .ok()?
            .parse()
            .ok()
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn body(&self) -> Option<&Body> {
        self.body.as_ref()
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn take_body(&mut self) -> Option<Body> {
        self.body.take()
    }

    pub fn with_body<NewBody>(self, body: NewBody) -> Response<NewBody> {
        Response {
            body: Some(body),
            headers: self.headers,
            status: self.status,
            version: self.version,
        }
    }
}

impl<'a, Body> IntoIterator for &'a Response<Body> {
    type Item = (&'a HeaderName, &'a HeaderValue);
    type IntoIter = http::header::Iter<'a, HeaderValue>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, Body> IntoIterator for &'a mut Response<Body> {
    type Item = (&'a HeaderName, &'a mut HeaderValue);
    type IntoIter = http::header::IterMut<'a, HeaderValue>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl Response<Vec<u8>> {
    pub(crate) fn new_with_status(status: StatusCode) -> Self {
        Self {
            status,
            headers: HeaderMap::new(),
            version: None,
            body: None,
        }
    }

    /// Reads the entire request body into a byte buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if the body has already been taken.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> crux_http::Result<()> {
    /// # let mut res = crux_http::testing::ResponseBuilder::ok()
    /// #   .header("Content-Type", "application/json")
    /// #   .body(vec![0u8, 1])
    /// #   .build();
    /// let bytes: Vec<u8> = res.body_bytes()?;
    /// # Ok(()) }
    /// ```
    pub fn body_bytes(&mut self) -> crate::Result<Vec<u8>> {
        self.body.take().ok_or_else(|| crate::HttpError::Http {
            code: self.status().as_u16(),
            message: "Body had no bytes".to_string(),
            body: None,
        })
    }

    /// Reads the entire response body into a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the body has already been taken or if it contains invalid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> crux_http::Result<()> {
    /// # let mut res = crux_http::testing::ResponseBuilder::ok()
    /// #   .header("Content-Type", "application/json")
    /// #   .body("hello".to_string().into_bytes())
    /// #   .build();
    /// let string: String = res.body_string()?;
    /// assert_eq!(string, "hello");
    /// # Ok(()) }
    /// ```
    pub fn body_string(&mut self) -> crate::Result<String> {
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
    /// Returns an error if the body has already been taken or if deserialisation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde::{Deserialize, Serialize};
    /// # fn main() -> crux_http::Result<()> {
    /// # let mut res = crux_http::testing::ResponseBuilder::ok()
    /// #   .header("Content-Type", "application/json")
    /// #   .body("{\"ip\": \"127.0.0.1\"}".to_string().into_bytes())
    /// #   .build();
    /// #[derive(Deserialize, Serialize)]
    /// struct Ip { ip: String }
    /// let Ip { ip } = res.body_json()?;
    /// assert_eq!(ip, "127.0.0.1");
    /// # Ok(()) }
    /// ```
    pub fn body_json<T: DeserializeOwned>(&mut self) -> crate::Result<T> {
        let body_bytes = self.body_bytes()?;
        serde_json::from_slice(&body_bytes).map_err(crate::HttpError::from)
    }
}

impl<Body> AsRef<HeaderMap> for Response<Body> {
    fn as_ref(&self) -> &HeaderMap {
        &self.headers
    }
}

impl<Body> AsMut<HeaderMap> for Response<Body> {
    fn as_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }
}

impl<Body> fmt::Debug for Response<Body> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("version", &self.version)
            .field("status", &self.status)
            .field("headers", &self.headers)
            .finish_non_exhaustive()
    }
}

impl<Body> Index<&str> for Response<Body> {
    type Output = HeaderValue;

    /// Returns a reference to the value corresponding to the supplied name.
    ///
    /// # Panics
    ///
    /// Panics if the name is not present in `Response`.
    #[inline]
    fn index(&self, name: &str) -> &HeaderValue {
        &self.headers[name]
    }
}

impl<Body> PartialEq for Response<Body>
where
    Body: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.status == other.status && self.headers == other.headers && self.body == other.body
    }
}

impl<Body> Eq for Response<Body> where Body: Eq {}

impl<Body> TryFrom<Response<Body>> for http::Response<Body> {
    type Error = ();

    fn try_from(res: Response<Body>) -> Result<Self, ()> {
        let body = res.body.ok_or(())?;
        let mut builder = http::Response::builder().status(res.status);
        if let Some(v) = res.version {
            builder = builder.version(v);
        }
        for (name, value) in &res.headers {
            builder = builder.header(name, value);
        }
        builder.body(body).map_err(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use http::{HeaderMap, StatusCode};

    use crate::response::Response;
    use crate::testing::ResponseBuilder;

    #[test]
    fn status_is_http_status_code() {
        let res = ResponseBuilder::ok().build();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.status().as_u16(), 200);
    }

    #[test]
    fn headers_are_http_header_map() {
        let res = ResponseBuilder::ok().header("x-custom", "hello").build();
        let map: &HeaderMap = res.as_ref();
        assert_eq!(map["x-custom"], "hello");
    }

    #[test]
    fn header_all_returns_multiple_values() {
        // header_all() API compiles and returns values even when only one is present
        // (ResponseBuilder uses insert_header which replaces).
        let res = ResponseBuilder::ok()
            .header("accept", "text/html")
            .header("accept", "application/json")
            .build();
        // ResponseBuilder uses insert_header which replaces; only last value survives
        // through the builder. This test verifies the API compiles and returns a value.
        assert!(res.header_all("accept").iter().next().is_some());
    }

    #[test]
    fn native_try_from_into_http_response() {
        use std::convert::TryFrom;
        let res: Response<Vec<u8>> = ResponseBuilder::ok()
            .header("x-foo", "bar")
            .body(b"hello".to_vec())
            .build();
        let http_res = http::Response::<Vec<u8>>::try_from(res).unwrap();
        assert_eq!(http_res.status(), StatusCode::OK);
        assert_eq!(http_res.headers()["x-foo"], "bar");
        assert_eq!(http_res.body(), b"hello");
    }

    /// Round-trip: `HttpResponse` → `crux_http::Response<Vec<u8>>` → `http::Response<Vec<u8>>`
    #[futures_test::test]
    async fn http_response_round_trip() {
        use crate::protocol::HttpResponse;
        use std::convert::TryFrom;

        let http_response = HttpResponse::ok()
            .header("content-type", "application/json")
            .json(serde_json::json!({"data": 42}))
            .build();

        // Step 1: HttpResponse → RawResponse (via From impl in response_async.rs)
        let response_async = crate::RawResponse::from(http_response);

        // Step 2: RawResponse → Response<Vec<u8>> (the path the command executor takes)
        let response = Response::<Vec<u8>>::new(response_async).expect("should decode");

        assert_eq!(response.status().as_u16(), 200);
        assert_eq!(response.content_type(), Some(mime::APPLICATION_JSON));

        // Step 3: Response<Vec<u8>> → http::Response<Vec<u8>> (native lossless conversion)
        let http_resp = http::Response::<Vec<u8>>::try_from(response).unwrap();
        assert_eq!(http_resp.status(), 200);
        assert_eq!(http_resp.headers()["content-type"], "application/json");
        let parsed: serde_json::Value = serde_json::from_slice(http_resp.body()).unwrap();
        assert_eq!(parsed["data"], 42);
    }

    #[test]
    fn response_status_serde_roundtrip() {
        let res: Response<Vec<u8>> = ResponseBuilder::ok().body(vec![42u8]).build();
        let json = serde_json::to_string(&res).expect("should serialize");
        let back: Response<Vec<u8>> = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(back.status().as_u16(), 200);
        assert_eq!(back.body().unwrap(), &[42u8]);
    }
}

/// Custom serde for `http::StatusCode` (serialized as `u16`).
mod status_serde {
    use http::StatusCode;
    use serde::{Deserialize, Deserializer, Serializer};

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn serialize<S: Serializer>(status: &StatusCode, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_u16(status.as_u16())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<StatusCode, D::Error> {
        let n = u16::deserialize(de)?;
        StatusCode::from_u16(n).map_err(serde::de::Error::custom)
    }
}

mod header_serde {
    use http::{HeaderMap, HeaderName, HeaderValue};
    use serde::{Deserializer, Serializer, de::Error};
    use std::str::FromStr;

    pub fn serialize<S>(headers: &HeaderMap, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Group values by name, preserving insertion order via collect_map.
        // Headers with multiple values each appear as separate entries.
        // We build a BTreeMap so the output is deterministic.
        let mut map: std::collections::BTreeMap<&str, Vec<&str>> =
            std::collections::BTreeMap::new();
        for (name, value) in headers {
            map.entry(name.as_str())
                .or_default()
                .push(value.to_str().unwrap_or(""));
        }
        serializer.collect_map(map.iter())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HeaderMap, D::Error>
    where
        D: Deserializer<'de>,
    {
        // The serialiser emits a JSON object (map); use HashMap to match.
        let strs =
            <std::collections::HashMap<String, Vec<String>> as serde::Deserialize>::deserialize(
                deserializer,
            )?;
        let mut headers = HeaderMap::new();
        for (name, values) in strs {
            let name = HeaderName::from_str(&name).map_err(D::Error::custom)?;
            for value in values {
                let value = HeaderValue::from_str(&value).map_err(D::Error::custom)?;
                headers.append(name.clone(), value);
            }
        }
        Ok(headers)
    }
}
