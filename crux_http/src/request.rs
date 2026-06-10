use crate::{Result, body::Body, middleware::Middleware};
use http::{HeaderMap, HeaderName, HeaderValue, Method};
use serde::Serialize;
use std::{fmt, ops::Index, sync::Arc};
use url::Url;

/// An HTTP request, returns a `Response`.
#[derive(Clone)]
pub struct Request {
    method: Method,
    url: Url,
    headers: HeaderMap,
    body: Body,
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
    /// use crux_http::{Url, Method};
    ///
    /// let url = Url::parse("https://httpbin.org/get")?;
    /// let req = crux_http::Request::new(Method::GET, url);
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn new(method: Method, url: Url) -> Self {
        Self {
            method,
            url,
            headers: HeaderMap::new(),
            body: Body::default(),
            middleware: None,
        }
    }

    /// Get the URL querystring.
    ///
    /// # Examples
    ///
    /// ```
    /// fn main() -> crux_http::Result<()> {
    /// use serde::{Deserialize, Serialize};
    /// use crux_http::{Request, Method, Url};
    /// #[derive(Serialize, Deserialize)]
    /// struct Index {
    ///     page: u32
    /// }
    ///
    /// let req = Request::new(Method::GET, Url::parse("https://httpbin.org/get?page=2")?);
    /// let Index { page } = req.query()?;
    /// assert_eq!(page, 2);
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    /// Returns an error if the query string could not be deserialized.
    pub fn query<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        let query = self.url.query().unwrap_or("");
        serde_qs::from_str(query).map_err(Into::into)
    }

    /// Set the URL querystring.
    ///
    /// # Examples
    ///
    /// ```
    /// fn main() -> crux_http::Result<()> {
    /// # use serde::{Deserialize, Serialize};
    /// # use crux_http::{Request, Method, Url};
    /// #[derive(Serialize, Deserialize)]
    /// struct Index {
    ///     page: u32
    /// }
    ///
    /// let query = Index { page: 2 };
    /// let mut req = Request::new(Method::GET, Url::parse("https://httpbin.org/get")?);
    /// req.set_query(&query)?;
    /// assert_eq!(req.url().query(), Some("page=2"));
    /// assert_eq!(req.url().as_str(), "https://httpbin.org/get?page=2");
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    /// Returns an error if the query string could not be serialized.
    pub fn set_query(&mut self, query: &impl Serialize) -> Result<()> {
        let qs = serde_qs::to_string(query)?;
        self.url.set_query(Some(&qs));
        Ok(())
    }

    /// Get an HTTP header.
    ///
    /// # Examples
    ///
    /// ```
    /// fn main() -> crux_http::Result<()> {
    /// # use crux_http::{Request, Method, Url};
    /// let mut req = Request::new(Method::GET, Url::parse("https://httpbin.org/get")?);
    /// req.set_header("X-Requested-With", "surf");
    /// assert_eq!(req.header("X-Requested-With").unwrap(), "surf");
    /// # Ok(()) }
    /// ```
    pub fn header(&self, name: impl http::header::AsHeaderName) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

    /// Get a mutable reference to a header.
    pub fn header_mut(
        &mut self,
        name: impl http::header::AsHeaderName,
    ) -> Option<&mut HeaderValue> {
        self.headers.get_mut(name)
    }

    /// Set an HTTP header, replacing any existing value.
    pub fn insert_header(
        &mut self,
        name: impl http::header::IntoHeaderName,
        value: impl AsRef<str>,
    ) {
        if let Ok(v) = HeaderValue::from_str(value.as_ref()) {
            self.headers.insert(name, v);
        }
    }

    /// Append a header to the headers.
    ///
    /// Unlike `insert_header` this function will not override the contents of a header, but insert
    /// a header if there aren't any. Or else append to the existing list of headers.
    pub fn append_header(
        &mut self,
        name: impl http::header::IntoHeaderName,
        value: impl AsRef<str>,
    ) {
        if let Ok(v) = HeaderValue::from_str(value.as_ref()) {
            self.headers.append(name, v);
        }
    }

    /// Remove a header.
    pub fn remove_header(&mut self, name: impl http::header::AsHeaderName) -> Option<HeaderValue> {
        self.headers.remove(name)
    }

    /// An iterator visiting all header pairs in arbitrary order.
    #[must_use]
    pub fn iter(&self) -> http::header::Iter<'_, HeaderValue> {
        self.headers.iter()
    }

    /// An iterator visiting all header pairs in arbitrary order, with mutable references to the
    /// values.
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

    /// Set an HTTP header.
    ///
    /// # Examples
    ///
    /// ```
    /// fn main() -> crux_http::Result<()> {
    /// # use crux_http::{Request, Method, Url};
    /// let mut req = Request::new(Method::GET, Url::parse("https://httpbin.org/get")?);
    /// req.set_header("X-Requested-With", "surf");
    /// assert_eq!(req.header("X-Requested-With").unwrap(), "surf");
    /// # Ok(()) }
    /// ```
    pub fn set_header(&mut self, key: impl http::header::IntoHeaderName, value: impl AsRef<str>) {
        self.insert_header(key, value);
    }

    /// Get the request HTTP method.
    ///
    /// # Examples
    ///
    /// ```
    /// fn main() -> crux_http::Result<()> {
    /// # use crux_http::{Request, Method, Url};
    /// let req = Request::new(Method::GET, Url::parse("https://httpbin.org/get")?);
    /// assert_eq!(req.method(), &Method::GET);
    /// # Ok(()) }
    /// ```
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Get the request url.
    ///
    /// # Examples
    ///
    /// ```
    /// fn main() -> crux_http::Result<()> {
    /// # use crux_http::{Request, Method, Url};
    /// let req = Request::new(Method::GET, Url::parse("https://httpbin.org/get")?);
    /// assert_eq!(req.url(), &Url::parse("https://httpbin.org/get")?);
    /// # Ok(()) }
    /// ```
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Get a mutable reference to the request url.
    ///
    /// This is useful for middleware that needs to rewrite the request URL.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn url_mut(&mut self) -> &mut Url {
        &mut self.url
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
    #[must_use]
    pub fn content_type(&self) -> Option<mime::Mime> {
        self.headers
            .get(http::header::CONTENT_TYPE)?
            .to_str()
            .ok()?
            .parse()
            .ok()
    }

    /// Set the request content type from a `Mime`.
    ///
    /// [Read more on MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types)
    pub fn set_content_type(&mut self, mime: &mime::Mime) {
        if let Ok(v) = HeaderValue::from_str(mime.as_ref()) {
            self.headers.insert(http::header::CONTENT_TYPE, v);
        }
    }

    /// Get the length of the body stream, if it has been set.
    ///
    /// This value is set when passing a fixed-size object into as the body.
    /// E.g. a string, or a buffer. Consumers of this API should check this
    /// value to decide whether to use `Chunked` encoding, or set the
    /// response length.
    #[allow(clippy::len_without_is_empty)]
    #[must_use]
    pub fn len(&self) -> Option<usize> {
        Some(self.body.len())
    }

    /// Returns `true` if the set length of the body stream is zero, `false`
    /// otherwise.
    #[must_use]
    pub fn is_empty(&self) -> Option<bool> {
        Some(self.body.is_empty())
    }

    /// Pass an `AsyncRead` stream as the request body.
    ///
    /// # Mime
    ///
    /// The encoding is set to `application/octet-stream`.
    pub fn set_body(&mut self, body: impl Into<Body>) {
        let body = body.into();
        if let Some(mime) = body.mime()
            && let Ok(v) = HeaderValue::from_str(mime.as_ref())
        {
            self.headers.insert(http::header::CONTENT_TYPE, v);
        }
        self.body = body;
    }

    /// Take the request body as a `Body`.
    ///
    /// This method can be called after the body has already been taken or read,
    /// but will return an empty `Body`.
    ///
    /// This is useful for consuming the body via an `AsyncReader` or `AsyncBufReader`.
    pub fn take_body(&mut self) -> Body {
        std::mem::take(&mut self.body)
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
    pub fn body_json(&mut self, json: &impl Serialize) -> Result<()> {
        self.set_body(Body::from_json(json)?);
        Ok(())
    }

    /// Pass a string as the request body.
    ///
    /// # Mime
    ///
    /// The `content-type` is set to `text/plain; charset=utf-8`.
    pub fn body_string(&mut self, string: String) {
        self.set_body(Body::from_string(string));
    }

    /// Pass bytes as the request body.
    ///
    /// # Mime
    ///
    /// The `content-type` is set to `application/octet-stream`.
    pub fn body_bytes(&mut self, bytes: impl AsRef<[u8]>) {
        self.set_body(Body::from(bytes.as_ref()));
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
    pub fn body_form(&mut self, form: &impl Serialize) -> Result<()> {
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
    /// fn main() -> crux_http::Result<()> {
    /// # use crux_http::{Request, Method, Url};
    /// let mut req = Request::new(Method::GET, Url::parse("https://httpbin.org/get")?);
    /// req.middleware(crux_http::middleware::Redirect::default());
    /// # Ok(()) }
    /// ```
    #[allow(clippy::missing_panics_doc)]
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

impl AsRef<HeaderMap> for Request {
    fn as_ref(&self) -> &HeaderMap {
        &self.headers
    }
}

impl AsMut<HeaderMap> for Request {
    fn as_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }
}

impl From<http::Request<Body>> for Request {
    fn from(req: http::Request<Body>) -> Self {
        let (parts, body) = req.into_parts();
        let url = parts
            .uri
            .to_string()
            .parse()
            .unwrap_or_else(|_| Url::parse("https://invalid.example.com").unwrap());
        Self {
            method: parts.method,
            url,
            headers: parts.headers,
            body,
            middleware: None,
        }
    }
}

impl From<Request> for http::Request<Body> {
    fn from(req: Request) -> Self {
        let mut builder = http::Request::builder()
            .method(req.method)
            .uri(req.url.as_str());
        for (name, value) in &req.headers {
            builder = builder.header(name, value);
        }
        builder.body(req.body).expect("valid request")
    }
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("method", &self.method)
            .field("url", &self.url.as_str())
            .finish_non_exhaustive()
    }
}

impl<'a> IntoIterator for &'a Request {
    type Item = (&'a HeaderName, &'a HeaderValue);
    type IntoIter = http::header::Iter<'a, HeaderValue>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.headers.iter()
    }
}

impl<'a> IntoIterator for &'a mut Request {
    type Item = (&'a HeaderName, &'a mut HeaderValue);
    type IntoIter = http::header::IterMut<'a, HeaderValue>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.headers.iter_mut()
    }
}

impl Index<&str> for Request {
    type Output = HeaderValue;

    /// Returns a reference to the value corresponding to the supplied name.
    ///
    /// # Panics
    ///
    /// Panics if the name is not present in `Request`.
    #[inline]
    fn index(&self, name: &str) -> &HeaderValue {
        &self.headers[name]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(url: &str) -> Request {
        Request::new(Method::GET, Url::parse(url).unwrap())
    }

    #[test]
    fn new_request_has_empty_body() {
        let req = get("https://example.com");
        assert!(req.is_empty() == Some(true));
        assert_eq!(req.len(), Some(0));
    }

    #[test]
    fn set_body_string_stores_bytes_and_content_type_header() {
        let mut req = get("https://example.com");
        req.set_body("hello");
        assert_eq!(req.len(), Some(5));
        // Content-Type header must be propagated to the headers map
        // so that iter() picks it up when building HttpRequest headers.
        let ct = req.content_type().expect("content type must be set");
        assert_eq!(ct, mime::TEXT_PLAIN_UTF_8);
    }

    #[test]
    fn set_body_bytes_sets_octet_stream_content_type() {
        let mut req = get("https://example.com");
        req.set_body(vec![1u8, 2, 3]);
        let ct = req.content_type().expect("content type must be set");
        assert_eq!(ct, mime::APPLICATION_OCTET_STREAM);
    }

    #[test]
    fn take_body_empties_body() {
        let mut req = get("https://example.com");
        req.set_body("world");
        let body = req.take_body();
        assert_eq!(body.into_bytes(), b"world");
        assert!(req.is_empty() == Some(true));
    }

    #[test]
    fn body_json_sets_application_json_content_type() {
        let mut req = get("https://example.com");
        req.body_json(&serde_json::json!({"key": "val"})).unwrap();
        let ct = req.content_type().expect("content type must be set");
        assert_eq!(ct, mime::APPLICATION_JSON);
        assert!(!req.is_empty().unwrap());
    }

    #[test]
    fn body_form_sets_form_urlencoded_content_type() {
        #[derive(serde::Serialize)]
        struct F {
            a: u32,
        }
        let mut req = get("https://example.com");
        req.body_form(&F { a: 1 }).unwrap();
        let ct = req.content_type().expect("content type must be set");
        assert_eq!(ct, mime::APPLICATION_WWW_FORM_URLENCODED);
    }

    #[test]
    fn method_is_http_method() {
        let req = Request::new(Method::POST, Url::parse("https://example.com").unwrap());
        assert_eq!(req.method(), &Method::POST);
    }

    #[test]
    fn from_http_request_roundtrip() {
        use crate::Body;
        let http_req = http::Request::builder()
            .method(Method::PUT)
            .uri("https://example.com/path")
            .header("x-test", "value")
            .body(Body::from("payload"))
            .unwrap();

        let req: Request = http_req.into();
        assert_eq!(req.method(), &Method::PUT);
        assert_eq!(req.header("x-test").unwrap().to_str().unwrap(), "value");
        assert_eq!(req.len(), Some(7));

        // And back again
        let back: http::Request<Body> = req.into();
        assert_eq!(back.method(), Method::PUT);
        assert_eq!(back.headers()["x-test"], "value");
    }

    #[test]
    fn url_mut_allows_url_mutation() {
        let mut req = get("https://example.com/old");
        *req.url_mut() = Url::parse("https://example.com/new").unwrap();
        assert_eq!(req.url().as_str(), "https://example.com/new");
    }
}
