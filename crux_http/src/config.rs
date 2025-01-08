//! Configuration for `HttpClient`s.

use std::{collections::HashMap, fmt::Debug};

use http_types::{
    headers::{HeaderName, HeaderValues, ToHeaderValues},
    Url,
};

use crate::Result;

/// Configuration for `crux_http::Http`s and their underlying HTTP client.
#[non_exhaustive]
#[derive(Clone, Debug, Default)]
pub struct Config {
    /// The base URL for a client. All request URLs will be relative to this URL.
    ///
    /// Note: a trailing slash is significant.
    /// Without it, the last path component is considered to be a “file” name
    /// to be removed to get at the “directory” that is used as the base.
    pub base_url: Option<Url>,
    /// Headers to be applied to every request made by this client.
    pub headers: HashMap<HeaderName, HeaderValues>,
}

impl Config {
    /// Construct new empty config.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Config {
    /// Adds a header to be added to every request by this config.
    ///
    /// Default: No extra headers.
    pub fn add_header(
        mut self,
        name: impl Into<HeaderName>,
        values: impl ToHeaderValues,
    ) -> Result<Self> {
        self.headers
            .insert(name.into(), values.to_header_values()?.collect());
        Ok(self)
    }

    /// Sets the base URL for this config. All request URLs will be relative to this URL.
    ///
    /// Note: a trailing slash is significant.
    /// Without it, the last path component is considered to be a “file” name
    /// to be removed to get at the “directory” that is used as the base.
    ///
    /// Default: `None` (internally).
    pub fn set_base_url(mut self, base: Url) -> Self {
        self.base_url = Some(base);
        self
    }
}
