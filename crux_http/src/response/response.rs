use super::{decode::decode_body, new_headers};
use http_types::{
    self,
    headers::{self, HeaderName, HeaderValues, ToHeaderValues},
    Mime, StatusCode, Version,
};

use http_types::{headers::CONTENT_TYPE, Headers};
use serde::de::DeserializeOwned;

use std::fmt;
use std::ops::Index;

/// An HTTP Response that will be passed to in a message to an apps update function
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Response<Body> {
    version: Option<http_types::Version>,
    status: http_types::StatusCode,
    #[serde(with = "header_serde")]
    headers: Headers,
    body: Option<Body>,
}

impl<Body> Response<Body> {
    /// Create a new instance.
    pub(crate) async fn new(mut res: super::ResponseAsync) -> crate::Result<Response<Vec<u8>>> {
        let body = res.body_bytes().await?;
        let status = res.status();

        if status.is_client_error() || status.is_server_error() {
            return Err(crate::HttpError::Http {
                code: status,
                message: status.to_string(),
                body: Some(body),
            });
        }

        let headers: &Headers = res.as_ref();
        let headers = headers.clone();

        Ok(Response {
            status: res.status(),
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
    /// assert_eq!(res.version(), Some(Version::Http1_1));
    /// ```
    pub fn version(&self) -> Option<Version> {
        self.version
    }

    /// Get a header.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # let res = crux_http::testing::ResponseBuilder::ok()
    /// #   .header("Content-Length", "1")
    /// #   .build();
    /// assert!(res.header("Content-Length").is_some());
    /// ```
    pub fn header(&self, name: impl Into<HeaderName>) -> Option<&HeaderValues> {
        self.headers.get(name)
    }

    /// Get an HTTP header mutably.
    pub fn header_mut(&mut self, name: impl Into<HeaderName>) -> Option<&mut HeaderValues> {
        self.headers.get_mut(name)
    }

    /// Remove a header.
    pub fn remove_header(&mut self, name: impl Into<HeaderName>) -> Option<HeaderValues> {
        self.headers.remove(name)
    }

    /// Insert an HTTP header.
    pub fn insert_header(&mut self, key: impl Into<HeaderName>, value: impl ToHeaderValues) {
        self.headers.insert(key, value);
    }

    /// Append an HTTP header.
    pub fn append_header(&mut self, key: impl Into<HeaderName>, value: impl ToHeaderValues) {
        self.headers.append(key, value);
    }

    /// An iterator visiting all header pairs in arbitrary order.
    #[must_use]
    pub fn iter(&self) -> headers::Iter<'_> {
        self.headers.iter()
    }

    /// An iterator visiting all header pairs in arbitrary order, with mutable references to the
    /// values.
    #[must_use]
    pub fn iter_mut(&mut self) -> headers::IterMut<'_> {
        self.headers.iter_mut()
    }

    /// An iterator visiting all header names in arbitrary order.
    #[must_use]
    pub fn header_names(&self) -> headers::Names<'_> {
        self.headers.names()
    }

    /// An iterator visiting all header values in arbitrary order.
    #[must_use]
    pub fn header_values(&self) -> headers::Values<'_> {
        self.headers.values()
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
    /// ```
    /// # let res = crux_http::testing::ResponseBuilder::ok()
    /// #   .header("Content-Type", "application/json")
    /// #   .build();
    /// use crux_http::http::mime;
    /// assert_eq!(res.content_type(), Some(mime::JSON));
    /// ```
    pub fn content_type(&self) -> Option<Mime> {
        self.header(CONTENT_TYPE)?.last().as_str().parse().ok()
    }

    pub fn body(&self) -> Option<&Body> {
        self.body.as_ref()
    }

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

impl Response<Vec<u8>> {
    pub(crate) fn new_with_status(status: http_types::StatusCode) -> Self {
        let headers = new_headers();

        Response {
            status,
            headers,
            version: None,
            body: None,
        }
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
            code: self.status(),
            message: "Body had no bytes".to_string(),
            body: None,
        })
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
    /// ```
    /// # use serde::{Deserialize, Serialize};
    /// # fn main() -> crux_http::Result<()> {
    /// # let mut res = crux_http::testing::ResponseBuilder::ok()
    /// #   .header("Content-Type", "application/json")
    /// #   .body("{\"ip\": \"127.0.0.1\"}".to_string().into_bytes())
    /// #   .build();
    /// #[derive(Deserialize, Serialize)]
    /// struct Ip {
    ///     ip: String
    /// }
    ///
    /// let Ip { ip } = res.body_json()?;
    /// assert_eq!(ip, "127.0.0.1");
    /// # Ok(()) }
    /// ```
    pub fn body_json<T: DeserializeOwned>(&mut self) -> crate::Result<T> {
        let body_bytes = self.body_bytes()?;
        serde_json::from_slice(&body_bytes).map_err(crate::HttpError::from)
    }
}

impl<Body> AsRef<http_types::Headers> for Response<Body> {
    fn as_ref(&self) -> &http_types::Headers {
        &self.headers
    }
}

impl<Body> AsMut<http_types::Headers> for Response<Body> {
    fn as_mut(&mut self) -> &mut http_types::Headers {
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

impl<Body> Index<HeaderName> for Response<Body> {
    type Output = HeaderValues;

    /// Returns a reference to the value corresponding to the supplied name.
    ///
    /// # Panics
    ///
    /// Panics if the name is not present in `Response`.
    #[inline]
    fn index(&self, name: HeaderName) -> &HeaderValues {
        &self.headers[name]
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
        &self.headers[name]
    }
}

impl<Body> PartialEq for Response<Body>
where
    Body: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
            && self.status == other.status
            && self.headers.iter().zip(other.headers.iter()).all(
                |((lhs_name, lhs_values), (rhs_name, rhs_values))| {
                    lhs_name == rhs_name
                        && lhs_values
                            .iter()
                            .zip(rhs_values.iter())
                            .all(|(lhs, rhs)| lhs == rhs)
                },
            )
            && self.body == other.body
    }
}

impl<Body> Eq for Response<Body> where Body: Eq {}

#[cfg(feature = "http-compat")]
impl<Body> TryInto<http::Response<Body>> for Response<Body> {
    type Error = ();

    fn try_into(self) -> Result<http::Response<Body>, Self::Error> {
        let mut response = http::Response::new(self.body.ok_or(())?);

        if let Some(version) = self.version {
            let version = match version {
                Version::Http0_9 => Some(http::Version::HTTP_09),
                Version::Http1_0 => Some(http::Version::HTTP_10),
                Version::Http1_1 => Some(http::Version::HTTP_11),
                Version::Http2_0 => Some(http::Version::HTTP_2),
                Version::Http3_0 => Some(http::Version::HTTP_3),
                _ => None,
            };

            if let Some(version) = version {
                *response.version_mut() = version;
            }
        }

        let mut headers = self.headers;
        headers_to_hyperium_headers(&mut headers, response.headers_mut());

        Ok(response)
    }
}

#[cfg(feature = "http-compat")]
fn headers_to_hyperium_headers(headers: &mut Headers, hyperium_headers: &mut http::HeaderMap) {
    for (name, values) in headers {
        let name = format!("{}", name).into_bytes();
        let name = http::header::HeaderName::from_bytes(&name).unwrap();

        for value in values.iter() {
            let value = format!("{}", value).into_bytes();
            let value = http::header::HeaderValue::from_bytes(&value).unwrap();
            hyperium_headers.append(&name, value);
        }
    }
}

mod header_serde {
    use crate::{http::Headers, response::new_headers};
    use http_types::headers::HeaderName;
    use serde::{de::Error, Deserializer, Serializer};

    pub fn serialize<S>(headers: &Headers, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_map(headers.iter().map(|(name, values)| {
            (
                name.as_str(),
                values.iter().map(|v| v.as_str()).collect::<Vec<_>>(),
            )
        }))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Headers, D::Error>
    where
        D: Deserializer<'de>,
    {
        let strs = <Vec<(String, Vec<String>)> as serde::Deserialize>::deserialize(deserializer)?;

        let mut headers = new_headers();

        for (name, values) in strs {
            let name = HeaderName::from_string(name).map_err(D::Error::custom)?;
            for value in values {
                headers.append(&name, value);
            }
        }

        Ok(headers)
    }
}
