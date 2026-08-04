use super::decode::decode_body;
use crate::{HttpError, Result};
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode, Version};
use serde::de::DeserializeOwned;
use std::{fmt, ops::Index};

/// An HTTP Response that will be passed to an app's update function.
///
/// # A `Response` never carries an error status
///
/// Holding one of these means the server did **not** reject the request. `crux_http`
/// converts every 4xx and 5xx response into an [`HttpError::Http`](crate::HttpError::Http)
/// — keeping the headers and body — and delivers it on the `Err` side, so
/// [`status`](Self::status) is always a 1xx, 2xx or 3xx. A `match` arm that checks it for
/// failure is dead code:
///
/// ```
/// # use crux_http::{HttpError, Response};
/// # fn saved() {}
/// # fn show_error(_message: &str) {}
/// fn on_result(result: crux_http::Result<Response<Vec<u8>>>) {
///     match result {
///         // this arm cannot see a 4xx or 5xx, so don't test the status here
///         Ok(_response) => saved(),
///         Err(error) => {
///             // the server's own message, e.g. {"error": "…"}, not just "409 Conflict"
///             let message = error
///                 .body_json::<serde_json::Value>()
///                 .ok()
///                 .and_then(|body| body["error"].as_str().map(str::to_string))
///                 .unwrap_or_else(|| error.to_string());
///             show_error(&message);
///         }
///     }
/// }
///
/// // a rejection, as a feature receives it
/// on_result(crux_http::testing::rejection(409, r#"{"error":"already booked"}"#));
/// ```
///
/// The matching testing rule: build success cases with
/// [`ResponseBuilder`](crate::testing::ResponseBuilder), and rejections with
/// [`rejection`](crate::testing::rejection).
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
    ///
    /// A 4xx or 5xx status is not a response as far as the app is concerned: it becomes
    /// [`HttpError::Http`], carrying the headers and body so the caller can still read what
    /// the server said. This is the single place that invariant is established — see the
    /// [type docs](Response) for what it means for app and test code.
    pub(crate) fn new(mut res: super::RawResponse) -> Result<Response<Vec<u8>>> {
        let body = res.body_bytes()?;
        let status = res.status();
        let headers = res.as_ref().clone();

        if status.is_client_error() || status.is_server_error() {
            return Err(HttpError::Http {
                code: status.as_u16(),
                message: status.to_string(),
                headers: Some(Box::new(headers)),
                body: Some(body),
            });
        }

        Ok(Response {
            status,
            headers,
            version: res.version(),
            body: Some(body),
        })
    }

    /// Get the HTTP status code.
    ///
    /// Never a client (4xx) or server (5xx) error: those are delivered as an
    /// [`HttpError::Http`](crate::HttpError::Http) on the `Err` side, not as a `Response`,
    /// so there is nothing to be learned by testing this for failure. Handle rejections
    /// there instead — see the [type docs](Response).
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
    /// Returns [`HttpError::BodyAlreadyTaken`] if the body has already been taken — this and
    /// the other `body_*` readers each take it, so only the first call can succeed.
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
    pub fn body_bytes(&mut self) -> Result<Vec<u8>> {
        self.body.take().ok_or(HttpError::BodyAlreadyTaken)
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
    pub fn body_json<T: DeserializeOwned>(&mut self) -> Result<T> {
        let body_bytes = self.body_bytes()?;
        serde_json::from_slice(&body_bytes).map_err(HttpError::from)
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

    fn try_from(res: Response<Body>) -> std::result::Result<Self, ()> {
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

    use crate::{
        HttpError, HttpResponse, RawResponse, response::Response, testing::ResponseBuilder,
    };

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
        let res = ResponseBuilder::ok()
            .header("accept", "text/html")
            .append_header("accept", "application/json")
            .build();
        let values: Vec<&str> = res
            .header_all("accept")
            .iter()
            .map(|v| v.to_str().unwrap())
            .collect();
        assert_eq!(values, ["text/html", "application/json"]);
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

        // Step 1: HttpResponse → RawResponse (via TryFrom impl in raw_response.rs)
        let response_async = RawResponse::try_from(http_response).expect("valid status");

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

    #[test]
    fn non_standard_status_499_becomes_http_error() {
        // 499 is a non-standard client error (client closed connection).
        // It arrives as HttpResponse from the shell, is converted to RawResponse,
        // and Response::new() converts it to HttpError::Http with the original code.
        let http_response = HttpResponse::status(499)
            .body(b"client closed connection".to_vec())
            .build();
        let raw = RawResponse::try_from(http_response).expect("499 is a valid status code");
        let result = Response::<Vec<u8>>::new(raw);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, HttpError::Http { code, .. } if code == 499));
    }

    #[test]
    fn non_standard_status_599_becomes_http_error() {
        let http_response = HttpResponse::status(599)
            .body(b"custom server error".to_vec())
            .build();
        let raw = RawResponse::try_from(http_response).expect("599 is a valid status code");
        let result = Response::<Vec<u8>>::new(raw);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, HttpError::Http { code, .. } if code == 599));
    }

    #[test]
    fn non_standard_4xx_status_preserves_code_in_error() {
        for status in [490, 491, 492, 493, 494, 495, 496, 497, 498, 499] {
            let http_response = HttpResponse::status(status).body(b"".to_vec()).build();
            let raw = RawResponse::try_from(http_response)
                .unwrap_or_else(|_| panic!("{status} is a valid status code"));
            let result = Response::<Vec<u8>>::new(raw);

            let err = result.expect_err("should be an error");
            assert!(
                matches!(err, HttpError::Http { code, .. } if code == status),
                "Expected status {status} to be preserved in HttpError, got: {err:?}"
            );
        }
    }

    #[test]
    fn response_serde_roundtrip_with_non_standard_status() {
        // 299 is non-standard but not an error, so it is a status a Response can hold —
        // a non-standard 4xx/5xx becomes HttpError::Http instead (see the tests above).
        let res: Response<Vec<u8>> = ResponseBuilder::with_status(299)
            .body(b"test".to_vec())
            .build();
        let json = serde_json::to_string(&res).expect("should serialize");
        let back: Response<Vec<u8>> = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(back.status().as_u16(), 299);
    }

    #[test]
    fn body_bytes_returns_error_when_body_already_taken() {
        let mut res: Response<Vec<u8>> = ResponseBuilder::ok().body(b"hello".to_vec()).build();
        let _ = res.body_bytes().unwrap();
        let err = res.body_bytes().expect_err("second call must fail");

        // Not a rejection: the server answered 200. It used to report itself as
        // `Http { code: 200, .. }`, which made `code()` unusable as a rejection test.
        assert!(matches!(err, HttpError::BodyAlreadyTaken), "got: {err:?}");
        assert_eq!(err.code(), None);
    }

    #[test]
    fn try_from_response_with_no_body_returns_err() {
        // `new_with_status` produces body: None (used for e.g. HEAD responses).
        let res = Response::<Vec<u8>>::new_with_status(StatusCode::OK);
        let result = http::Response::<Vec<u8>>::try_from(res);
        assert!(result.is_err(), "TryFrom must return Err when body is None");
    }

    #[test]
    fn multi_value_headers_survive_serde_roundtrip() {
        let res: Response<Vec<u8>> = ResponseBuilder::ok()
            .header("set-cookie", "a=1")
            .append_header("set-cookie", "b=2")
            .body(b"".to_vec())
            .build();

        let json = serde_json::to_string(&res).expect("should serialize");
        let back: Response<Vec<u8>> = serde_json::from_str(&json).expect("should deserialize");

        let values: Vec<&str> = back
            .header_all("set-cookie")
            .iter()
            .map(|v| v.to_str().unwrap())
            .collect();
        assert_eq!(
            values.len(),
            2,
            "both Set-Cookie values must survive serde: {values:?}"
        );
        assert!(values.contains(&"a=1"));
        assert!(values.contains(&"b=2"));
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
