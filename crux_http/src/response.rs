use crate::http::{
    self,
    headers::{self, HeaderName, HeaderValues, ToHeaderValues},
    Body, Error, Mime, StatusCode, Version,
};

use futures_util::io::AsyncRead;
use serde::de::DeserializeOwned;

use std::fmt;
use std::io;
use std::ops::Index;
use std::pin::Pin;
use std::task::{Context, Poll};

pin_project_lite::pin_project! {
    pub struct Response {
        #[pin]
        res: crate::http::Response,
    }
}

impl Response {
    /// Create a new instance.
    pub(crate) fn new(res: http::Response) -> Self {
        Self { res }
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
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let mut res = crux_http::get("https://httpbin.org/get").await?;
    /// let bytes: Vec<u8> = res.body_bytes().await?;
    /// # Ok(()) }
    /// ```
    pub async fn body_bytes(&mut self) -> crate::Result<Vec<u8>> {
        self.res.body_bytes().await
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
        let bytes = self.body_bytes().await?;
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
    pub async fn body_json<T: DeserializeOwned>(&mut self) -> crate::Result<T> {
        let body_bytes = self.body_bytes().await?;
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

impl From<http::Response> for Response {
    fn from(response: http::Response) -> Self {
        Self::new(response)
    }
}

#[allow(clippy::from_over_into)]
impl Into<http::Response> for Response {
    fn into(self) -> http::Response {
        self.res
    }
}

impl AsRef<http::Headers> for Response {
    fn as_ref(&self) -> &http::Headers {
        self.res.as_ref()
    }
}

impl AsMut<http::Headers> for Response {
    fn as_mut(&mut self) -> &mut http::Headers {
        self.res.as_mut()
    }
}

impl AsRef<http::Response> for Response {
    fn as_ref(&self) -> &http::Response {
        &self.res
    }
}

impl AsMut<http::Response> for Response {
    fn as_mut(&mut self) -> &mut http::Response {
        &mut self.res
    }
}

impl AsyncRead for Response {
    #[allow(missing_doc_code_examples)]
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize, io::Error>> {
        Pin::new(&mut self.res).poll_read(cx, buf)
    }
}

impl fmt::Debug for Response {
    #[allow(missing_doc_code_examples)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("response", &self.res)
            .finish()
    }
}

impl Index<HeaderName> for Response {
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

impl Index<&str> for Response {
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

/// An error occurred while decoding a response body to a string.
///
/// The error carries the encoding that was used to attempt to decode the body, and the raw byte
/// contents of the body. This can be used to treat un-decodable bodies specially or to implement a
/// fallback parsing strategy.
#[derive(Clone)]
pub struct DecodeError {
    /// The name of the encoding that was used to try to decode the input.
    pub encoding: String,
    /// The input data as bytes.
    pub data: Vec<u8>,
}

// Override debug output so you don't get each individual byte in `data` printed out separately,
// because it can be many megabytes large. The actual content is not that interesting anyways
// and can be accessed manually if it is required.
impl fmt::Debug for DecodeError {
    #[allow(missing_doc_code_examples)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DecodeError")
            .field("encoding", &self.encoding)
            // Perhaps we can output the first N bytes of the response in the future
            .field("data", &format!("{} bytes", self.data.len()))
            .finish()
    }
}

impl fmt::Display for DecodeError {
    #[allow(missing_doc_code_examples)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "could not decode body as {}", &self.encoding)
    }
}

impl std::error::Error for DecodeError {}

/// Check if an encoding label refers to the UTF-8 encoding.
#[allow(dead_code)]
fn is_utf8_encoding(encoding_label: &str) -> bool {
    encoding_label.eq_ignore_ascii_case("utf-8")
        || encoding_label.eq_ignore_ascii_case("utf8")
        || encoding_label.eq_ignore_ascii_case("unicode-1-1-utf-8")
}

/// Decode a response body as utf-8.
///
/// # Errors
///
/// If the body cannot be decoded as utf-8, this function returns an `std::io::Error` of kind
/// `std::io::ErrorKind::InvalidData`, carrying a `DecodeError` struct.
#[cfg(not(feature = "encoding"))]
fn decode_body(bytes: Vec<u8>, content_encoding: Option<&str>) -> Result<String, Error> {
    if is_utf8_encoding(content_encoding.unwrap_or("utf-8")) {
        Ok(String::from_utf8(bytes).map_err(|err| {
            let err = DecodeError {
                encoding: "utf-8".to_string(),
                data: err.into_bytes(),
            };
            io::Error::new(io::ErrorKind::InvalidData, err)
        })?)
    } else {
        let err = DecodeError {
            encoding: "utf-8".to_string(),
            data: bytes,
        };
        Err(io::Error::new(io::ErrorKind::InvalidData, err).into())
    }
}

/// Decode a response body as the given content type.
///
/// If the input bytes are valid utf-8, this does not make a copy.
///
/// # Errors
///
/// If an unsupported encoding is requested, or the body does not conform to the requested
/// encoding, this function returns an `std::io::Error` of kind `std::io::ErrorKind::InvalidData`,
/// carrying a `DecodeError` struct.
#[cfg(all(feature = "encoding", not(target_arch = "wasm32")))]
fn decode_body(bytes: Vec<u8>, content_encoding: Option<&str>) -> Result<String, Error> {
    use encoding_rs::Encoding;
    use std::borrow::Cow;

    let content_encoding = content_encoding.unwrap_or("utf-8");
    if let Some(encoding) = Encoding::for_label(content_encoding.as_bytes()) {
        let (decoded, encoding_used, failed) = encoding.decode(&bytes);
        if failed {
            let err = DecodeError {
                encoding: encoding_used.name().into(),
                data: bytes,
            };
            Err(io::Error::new(io::ErrorKind::InvalidData, err).into())
        } else {
            Ok(match decoded {
                // If encoding_rs returned a `Cow::Borrowed`, the bytes are guaranteed to be valid
                // UTF-8, by virtue of being UTF-8 or being in the subset of ASCII that is the same
                // in UTF-8.
                Cow::Borrowed(_) => unsafe { String::from_utf8_unchecked(bytes) },
                Cow::Owned(string) => string,
            })
        }
    } else {
        let err = DecodeError {
            encoding: content_encoding.to_string(),
            data: bytes,
        };
        Err(io::Error::new(io::ErrorKind::InvalidData, err).into())
    }
}

/// Decode a response body as the given content type.
///
/// This always makes a copy. (It could be optimized to avoid the copy if the encoding is utf-8.)
///
/// # Errors
///
/// If an unsupported encoding is requested, or the body does not conform to the requested
/// encoding, this function returns an `std::io::Error` of kind `std::io::ErrorKind::InvalidData`,
/// carrying a `DecodeError` struct.
#[cfg(all(feature = "encoding", target_arch = "wasm32"))]
fn decode_body(mut bytes: Vec<u8>, content_encoding: Option<&str>) -> Result<String, Error> {
    use web_sys::TextDecoder;

    // Encoding names are always valid ASCII, so we can avoid including casing mapping tables
    let content_encoding = content_encoding.unwrap_or("utf-8").to_ascii_lowercase();
    if is_utf8_encoding(&content_encoding) {
        return String::from_utf8(bytes)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err).into());
    }

    let decoder = TextDecoder::new_with_label(&content_encoding).unwrap();

    Ok(decoder.decode_with_u8_array(&mut bytes).map_err(|_| {
        let err = DecodeError {
            encoding: content_encoding.to_string(),
            data: bytes,
        };
        io::Error::new(io::ErrorKind::InvalidData, err)
    })?)
}

#[cfg(test)]
mod decode_tests {
    use super::decode_body;

    #[test]
    fn utf8() {
        let input = "Rød grød med fløde";
        assert_eq!(
            decode_body(input.as_bytes().to_vec(), Some("utf-8")).unwrap(),
            input,
            "Parses utf-8"
        );
    }

    #[test]
    fn default_utf8() {
        let input = "Rød grød med fløde";
        assert_eq!(
            decode_body(input.as_bytes().to_vec(), None).unwrap(),
            input,
            "Defaults to utf-8"
        );
    }

    #[test]
    fn euc_kr() {
        let input = vec![
            0xb3, 0xbb, 0x20, 0xc7, 0xb0, 0xc0, 0xb8, 0xb7, 0xce, 0x20, 0xb5, 0xb9, 0xbe, 0xc6,
            0xbf, 0xc0, 0xb6, 0xf3, 0x2c, 0x20, 0xb3, 0xbb, 0x20, 0xbe, 0xc8, 0xbf, 0xa1, 0xbc,
            0xad, 0x20, 0xc0, 0xe1, 0xb5, 0xe9, 0xb0, 0xc5, 0xb6, 0xf3,
        ];

        let result = decode_body(input, Some("euc-kr"));
        if cfg!(feature = "encoding") {
            assert_eq!(result.unwrap(), "내 품으로 돌아오라, 내 안에서 잠들거라");
        } else {
            assert!(result.is_err(), "Only utf-8 is supported");
        }
    }
}
