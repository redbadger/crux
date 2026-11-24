//! Configuration for `HttpClient`s.

use crate::prelude::*;
use http::{HeaderMap, HeaderValue};
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
    /// Returns an error if the header value is invalid.
    pub fn add_header(
        mut self,
        name: impl http::header::IntoHeaderName,
        value: impl AsRef<str>,
    ) -> Result<Self> {
        let value =
            HeaderValue::from_str(value.as_ref()).map_err(|e| HttpError::Io(e.to_string()))?;
        self.headers.append(name, value);
        Ok(self)
    }

    /// Sets the base URL for this config.
    #[must_use]
    pub fn set_base_url(mut self, base: Url) -> Self {
        self.base_url = Some(base);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_header_stores_valid_value() {
        let config = Config::default().add_header("x-api-key", "secret").unwrap();
        let val = config.headers.get("x-api-key").unwrap();
        assert_eq!(val.to_str().unwrap(), "secret");
    }

    #[test]
    fn add_header_called_twice_preserves_both_values() {
        let config = Config::default()
            .add_header("accept", "text/html")
            .unwrap()
            .add_header("accept", "application/json")
            .unwrap();
        let values: Vec<&str> = config
            .headers
            .get_all("accept")
            .iter()
            .map(|v| v.to_str().unwrap())
            .collect();
        assert_eq!(values, ["text/html", "application/json"]);
    }

    #[test]
    fn add_header_rejects_invalid_value() {
        // Control characters (other than tab) are rejected by HeaderValue::from_str.
        let result = Config::default().add_header("x-bad", "val\x00ue");
        assert!(
            matches!(result, Err(HttpError::Io(_))),
            "invalid header value must return HttpError::Io"
        );
    }
}
