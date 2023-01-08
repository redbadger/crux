use http::headers::{HeaderName, ToHeaderValues};

use crate::http;

use crate::response::Response;

/// Allows users to build an http response.
///
/// This is mostly expected to be useful in tests rather than application code.
pub struct ResponseBuilder<Body> {
    response: Response<Body>,
}

impl ResponseBuilder<Vec<u8>> {
    /// Constructs a new ResponseBuilder with the 200 OK status code.
    pub fn ok() -> ResponseBuilder<Vec<u8>> {
        let response = Response::new_with_status(http::StatusCode::Ok);

        ResponseBuilder { response }
    }
}

impl<Body> ResponseBuilder<Body> {
    /// Sets the body of the Response
    pub fn with_body<NewBody>(self, body: NewBody) -> ResponseBuilder<NewBody> {
        let response = self.response.with_body(body);
        ResponseBuilder { response }
    }

    /// Sets a header on the response.
    pub fn header(mut self, key: impl Into<HeaderName>, value: impl ToHeaderValues) -> Self {
        self.response.insert_header(key, value);
        self
    }

    /// Builds the response
    pub fn build(self) -> Response<Body> {
        self.response
    }
}
