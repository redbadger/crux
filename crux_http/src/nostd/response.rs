//! `spike(no_std)`: the [`Response`] an app sees, without `http` types.
//!
//! The std `Response` keeps an `http::StatusCode` and `http::HeaderMap`; `http` cannot be
//! built without std. This one keeps the status as a `u16` and the headers as the shell
//! sent them. The names and call shapes an app uses (`status()`, `header()`, `body()`,
//! `take_body()`, `body_json()`, …) are the same; the types behind them are not.

use core::fmt;

use serde::de::DeserializeOwned;

#[allow(unused_imports)]
use crate::prelude::*;
use crate::{
    HttpError, Result,
    protocol::{HttpHeader, HttpResponse},
};

/// An HTTP response, as seen by the app.
///
/// As with the std `Response`, a 4xx or 5xx response never becomes one: it is turned into
/// [`HttpError::Http`] first, so [`Response::status`] is never an error status.
#[derive(Clone)]
pub struct Response<Body> {
    status: u16,
    headers: Vec<HttpHeader>,
    body: Option<Body>,
}

impl<Body> Response<Body> {
    pub(crate) fn new(response: HttpResponse) -> Result<Response<Vec<u8>>> {
        let HttpResponse {
            status,
            headers,
            body,
        } = response;

        // `http::StatusCode::from_u16` accepts 100..=999, so the same range is valid here.
        if !(100..=999).contains(&status) {
            return Err(HttpError::InvalidStatusCode(status));
        }

        if (400..600).contains(&status) {
            return Err(HttpError::Http {
                code: status,
                // The std version uses `StatusCode`'s display, e.g. "409 Conflict"; without
                // `http` there is no reason-phrase table, so it is just the number.
                message: format!("{status}"),
                headers,
                body,
            });
        }

        Ok(Response {
            status,
            headers,
            body: Some(body),
        })
    }

    /// The status code, as a plain `u16`.
    #[must_use]
    pub const fn status(&self) -> u16 {
        self.status
    }

    /// A header, matched case-insensitively; the first one if there are several.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(name))
            .map(|header| header.value.as_str())
    }

    /// All headers, as the shell sent them.
    #[must_use]
    pub fn headers(&self) -> &[HttpHeader] {
        &self.headers
    }

    /// The `content-type` header, unparsed (there is no `mime` without std).
    #[must_use]
    pub fn content_type(&self) -> Option<&str> {
        self.header("content-type")
    }

    #[must_use]
    pub const fn body(&self) -> Option<&Body> {
        self.body.as_ref()
    }

    pub const fn take_body(&mut self) -> Option<Body> {
        self.body.take()
    }

    pub fn with_body<NewBody>(self, body: NewBody) -> Response<NewBody> {
        Response {
            status: self.status,
            headers: self.headers,
            body: Some(body),
        }
    }
}

impl Response<Vec<u8>> {
    /// Take the body as bytes.
    ///
    /// # Errors
    /// [`HttpError::BodyAlreadyTaken`] if it has been taken already.
    pub fn body_bytes(&mut self) -> Result<Vec<u8>> {
        self.body.take().ok_or(HttpError::BodyAlreadyTaken)
    }

    /// Take the body as a UTF-8 string. Unlike std (which uses `encoding_rs` and the
    /// content type's charset), only UTF-8 is supported.
    ///
    /// # Errors
    /// [`HttpError::BodyAlreadyTaken`], or [`HttpError::Io`] if the body is not UTF-8
    /// (the same variant and message the std version uses).
    pub fn body_string(&mut self) -> Result<String> {
        let bytes = self.body_bytes()?;
        String::from_utf8(bytes)
            .map_err(|_| HttpError::Io("could not decode body as utf-8".to_string()))
    }

    /// Take the body and deserialize it from JSON.
    ///
    /// # Errors
    /// [`HttpError::BodyAlreadyTaken`], or [`HttpError::Json`] if it is not valid JSON for `T`.
    pub fn body_json<T: DeserializeOwned>(&mut self) -> Result<T> {
        let bytes = self.body_bytes()?;
        serde_json::from_slice(&bytes).map_err(HttpError::from)
    }
}

impl TryFrom<HttpResponse> for Response<Vec<u8>> {
    type Error = HttpError;

    /// Convert a shell's protocol-level response into the [`Response`] an app sees.
    fn try_from(response: HttpResponse) -> Result<Self> {
        Self::new(response)
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

impl<Body> fmt::Debug for Response<Body> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("status", &self.status)
            .field("headers", &self.headers)
            .finish_non_exhaustive()
    }
}
