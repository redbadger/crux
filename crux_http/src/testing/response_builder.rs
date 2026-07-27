use http::{HeaderValue, StatusCode};

use crate::response::Response;

/// Allows users to build an http response.
///
/// This is mostly expected to be useful in tests rather than application code.
///
/// Only responses a feature can actually receive are buildable: a
/// [`Response`](crate::Response) never carries a 4xx or 5xx status, because `crux_http`
/// converts those to [`HttpError::Http`](crate::HttpError::Http) before the app sees
/// them. Use [`rejection`](super::rejection) to build a rejection.
pub struct ResponseBuilder<Body> {
    response: Response<Body>,
}

impl ResponseBuilder<Vec<u8>> {
    /// Constructs a new `ResponseBuilder` with the 200 OK status code.
    #[must_use]
    pub fn ok() -> Self {
        Self::with_status(200)
    }

    /// Constructs a new `ResponseBuilder` with the specified status code.
    ///
    /// # Panics
    ///
    /// Panics if `status` is outside the valid HTTP range (100–999), or if it is a client
    /// (4xx) or server (5xx) error. Such a response is not a state any app can observe —
    /// `crux_http` delivers it as an [`HttpError::Http`](crate::HttpError::Http) on the
    /// `Err` side, so a test that builds one asserts against a branch the app can never
    /// take. Build the rejection a feature really receives with
    /// [`rejection`](super::rejection):
    ///
    /// ```
    /// # use crux_http::testing::rejection;
    /// let result = rejection::<Vec<u8>>(409, r#"{"error":"already booked"}"#);
    /// assert_eq!(result.unwrap_err().code(), Some(409));
    /// ```
    #[must_use]
    pub fn with_status(status: u16) -> Self {
        let status = StatusCode::from_u16(status).expect(
            "ResponseBuilder::with_status called with an out-of-range code (must be 100–999)",
        );
        assert!(
            !status.is_client_error() && !status.is_server_error(),
            "ResponseBuilder::with_status called with {status}, but a Response never carries a \
             client (4xx) or server (5xx) status — those reach the app as \
             Err(HttpError::Http {{ .. }}). Use crux_http::testing::rejection instead."
        );
        let response = Response::new_with_status(status);
        Self { response }
    }
}

impl<Body> ResponseBuilder<Body> {
    /// Sets the body of the Response.
    pub fn body<NewBody>(self, body: NewBody) -> ResponseBuilder<NewBody> {
        let response = self.response.with_body(body);
        ResponseBuilder { response }
    }

    /// Sets a header on the response, replacing any existing value for that name.
    ///
    /// # Panics
    /// Panics if `value` is not a valid header value.
    #[must_use]
    pub fn header(
        mut self,
        name: impl http::header::IntoHeaderName,
        value: impl AsRef<str>,
    ) -> Self {
        let value = HeaderValue::from_str(value.as_ref()).expect("invalid header value");
        self.response.insert_header(name, value);
        self
    }

    /// Appends a header value, keeping any existing values for that name.
    ///
    /// Use this when building responses with multiple values for the same header
    /// (e.g. `Set-Cookie`).
    ///
    /// # Panics
    /// Panics if `value` is not a valid header value.
    #[must_use]
    pub fn append_header(
        mut self,
        name: impl http::header::IntoHeaderName,
        value: impl AsRef<str>,
    ) -> Self {
        let value = HeaderValue::from_str(value.as_ref()).expect("invalid header value");
        self.response.append_header(name, value);
        self
    }

    /// Builds the response.
    pub fn build(self) -> Response<Body> {
        self.response
    }
}

#[cfg(test)]
mod tests {
    use super::ResponseBuilder;

    #[test]
    fn builds_any_non_error_status() {
        for status in [200, 201, 204, 299, 302, 304] {
            assert_eq!(
                ResponseBuilder::with_status(status).build().status(),
                status
            );
        }
    }

    #[test]
    #[should_panic(expected = "a Response never carries a client (4xx) or server (5xx) status")]
    fn refuses_a_client_error_status() {
        let _ = ResponseBuilder::with_status(409);
    }

    #[test]
    #[should_panic(expected = "a Response never carries a client (4xx) or server (5xx) status")]
    fn refuses_a_server_error_status() {
        let _ = ResponseBuilder::with_status(503);
    }
}
