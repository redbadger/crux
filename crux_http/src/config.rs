//! Configuration for `HttpClient`s.

use http::{HeaderMap, HeaderName, HeaderValue};
use std::fmt::Debug;
use url::Url;

use crate::{HttpError, Result};

/// Configuration for `crux_http::Http`s and their underlying HTTP client.
#[non_exhaustive]
#[derive(Clone, Debug, Default)]
pub struct Config {
    /// The base URL for a client. All request URLs will be relative to this URL.
    ///
    /// Note: a trailing slash is significant.
    /// Without it, the last path component is considered to be a "file" name
    /// to be removed to get at the "directory" that is used as the base.
    pub base_url: Option<Url>,
    /// Headers to be applied to every request made by this client.
    pub headers: HeaderMap,
}

impl Config {
    /// Construct new empty config.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Config {
    /// Adds a header to be added to every request by this config.
    ///
    /// Default: No extra headers.
    ///
    /// # Errors
    /// Returns an error if the header name or value is invalid.
    pub fn add_header(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Result<Self> {
        use std::str::FromStr;
        let name = HeaderName::from_str(name.as_ref()).map_err(|e| HttpError::Io(e.to_string()))?;
        let value =
            HeaderValue::from_str(value.as_ref()).map_err(|e| HttpError::Io(e.to_string()))?;
        self.headers.insert(name, value);
        Ok(self)
    }

    /// Sets the base URL for this config.
    #[must_use]
    pub fn set_base_url(mut self, base: Url) -> Self {
        self.base_url = Some(base);
        self
    }
}
