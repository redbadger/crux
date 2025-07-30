use super::decode::decode_body;
use crate::{
    Mime, StatusCode, Version,
    header::{self, HeaderMap, HeaderName, HeaderValue},
};
use facet::Facet;

use serde::de::DeserializeOwned;

use std::fmt;
use std::ops::Index;

/// An HTTP Response that will be passed to in a message to an apps update function
#[derive(Facet, Clone, serde::Serialize, serde::Deserialize)]
pub struct Response<Body> {
    version: Option<Version>,
    status: StatusCode,
    #[serde(with = "header_serde")]
    headers: HeaderMap,
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

        let headers: &HeaderMap = res.as_ref();
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
    pub fn header(&self, name: impl Into<HeaderName>) -> Option<&HeaderMap> {
        self.headers.get(name)
    }

    /// Get an HTTP header mutably.
    pub fn header_mut(&mut self, name: impl Into<HeaderName>) -> Option<&mut HeaderMap> {
        self.headers.get_mut(name)
    }

    /// Remove a header.
    pub fn remove_header(&mut self, name: impl Into<HeaderName>) -> Option<HeaderMap> {
        self.headers.remove(name)
    }

    /// Insert an HTTP header.
    pub fn insert_header(&mut self, key: impl Into<HeaderName>, value: impl Into<HeaderValue>) {
        self.headers.insert(key, value);
    }

    /// Append an HTTP header.
    pub fn append_header(&mut self, key: impl Into<HeaderName>, value: impl Into<HeaderValue>) {
        self.headers.append(key, value);
    }

    /// An iterator visiting all header pairs in arbitrary order.
    #[must_use]
    pub fn iter(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        self.headers.iter()
    }

    /// An iterator visiting all header pairs in arbitrary order, with mutable references to the
    /// values.
    #[must_use]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&HeaderName, &mut HeaderValue)> {
        self.headers.iter_mut()
    }

    /// An iterator visiting all header names in arbitrary order.
    #[must_use]
    pub fn header_names(&self) -> impl Iterator<Item = &HeaderName> {
        self.headers.keys()
    }

    /// An iterator visiting all header values in arbitrary order.
    #[must_use]
    pub fn header_values(&self) -> impl Iterator<Item = &HeaderValue> {
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
        self.header(header::CONTENT_TYPE)?
            .last()
            .as_str()
            .parse()
            .ok()
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

impl<'a, Body> IntoIterator for &'a Response<Body> {
    type Item = (&'a header::HeaderName, &'a header::HeaderValue);
    type IntoIter = header::Iter<'a, HeaderValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, Body> IntoIterator for &'a mut Response<Body> {
    type Item = (&'a header::HeaderName, &'a mut header::HeaderValue);
    type IntoIter = header::IterMut<'a, header::HeaderValue>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl Response<Vec<u8>> {
    pub(crate) fn new_with_status(status: crate::StatusCode) -> Self {
        let headers = HeaderMap::new();

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
            .map(std::string::ToString::to_string);
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

impl<Body> AsRef<header::HeaderMap> for Response<Body> {
    fn as_ref(&self) -> &header::HeaderMap {
        &self.headers
    }
}

impl<Body> AsMut<header::HeaderMap> for Response<Body> {
    fn as_mut(&mut self) -> &mut header::HeaderMap {
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
    type Output = HeaderValue;

    /// Returns a reference to the value corresponding to the supplied name.
    ///
    /// # Panics
    ///
    /// Panics if the name is not present in `Response`.
    #[inline]
    fn index(&self, name: HeaderName) -> &HeaderValue {
        &self.headers[name]
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

mod header_serde {
    use crate::header::{HeaderMap, HeaderName, HeaderValue};
    use serde::{Deserializer, Serializer, de::Error};

    pub fn serialize<S>(headers: &HeaderMap, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_map(headers.iter().map(|(name, values)| {
            (
                name.as_str(),
                values.iter().map(HeaderValue::as_str).collect::<Vec<_>>(),
            )
        }))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HeaderMap, D::Error>
    where
        D: Deserializer<'de>,
    {
        let strs = <Vec<(String, Vec<String>)> as serde::Deserialize>::deserialize(deserializer)?;

        let mut headers = HeaderMap::new();

        for (name, values) in strs {
            let name = HeaderName::from_string(name).map_err(D::Error::custom)?;
            for value in values {
                headers.append(&name, value);
            }
        }

        Ok(headers)
    }
}
