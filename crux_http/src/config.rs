//! Configuration for `HttpClient`s.

use std::sync::Arc;
use std::{collections::HashMap, fmt::Debug, time::Duration};

use http_types::headers::{HeaderName, HeaderValues, ToHeaderValues};

use crate::http::Url;
use crate::Result;

/// Configuration for `crux_http::Client`s and their underlying HTTP clients.
///
/// ```
/// use std::convert::TryInto;
/// use crux_http::{Client, Config, Url};
///
/// # #[async_std::main]
/// # async fn main() -> crux_http::Result<()> {
/// let client: Client = Config::new()
///     .set_base_url(Url::parse("https://example.org")?)
///     .try_into()?;
///
/// let mut response = client.get("/").await?;
///
/// println!("{}", response.body_string().await?);
/// # Ok(())
/// # }
/// ```
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
    /// Adds a header to be added to every request by this client.
    ///
    /// Default: No extra headers.
    ///
    /// ```
    /// use std::convert::TryInto;
    /// use crux_http::{Client, Config};
    /// use crux_http::http::auth::BasicAuth;
    ///
    /// # fn main() -> crux_http::Result<()> {
    /// let auth = BasicAuth::new("Username", "Password");
    ///
    /// let client: Client = Config::new()
    ///     .add_header(auth.name(), auth.value())?
    ///     .try_into()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_header(
        mut self,
        name: impl Into<HeaderName>,
        values: impl ToHeaderValues,
    ) -> Result<Self> {
        self.headers
            .insert(name.into(), values.to_header_values()?.collect());
        Ok(self)
    }

    /// Sets the base URL for this client. All request URLs will be relative to this URL.
    ///
    /// Note: a trailing slash is significant.
    /// Without it, the last path component is considered to be a “file” name
    /// to be removed to get at the “directory” that is used as the base.
    ///
    /// Default: `None` (internally).
    ///
    /// ```
    /// use std::convert::TryInto;
    /// use crux_http::{Client, Config, Url};
    ///
    /// # fn main() -> crux_http::Result<()> {
    /// let client: Client = Config::new()
    ///     .set_base_url(Url::parse("https://example.org")?)
    ///     .try_into()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_base_url(mut self, base: Url) -> Self {
        self.base_url = Some(base);
        self
    }
}
