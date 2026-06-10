use http::{HeaderValue, StatusCode};

use crate::response::Response;

/// Allows users to build an http response.
///
/// This is mostly expected to be useful in tests rather than application code.
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
    #[must_use]
    pub fn with_status(status: u16) -> Self {
        let status = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
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
