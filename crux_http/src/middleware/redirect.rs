//! HTTP Redirect middleware.
//!
//! # Examples
//!
//! ```no_run
//! # use crux_core::macros::effect;
//! # use crux_http::HttpRequest;
//! # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
//! # #[effect]
//! # #[allow(unused)]
//! # enum Effect { Http(HttpRequest) }
//! # type Http = crux_http::Http<Effect, Event>;
//!
//! Http::get("https://httpbin.org/redirect/2")
//!     .middleware(crux_http::middleware::Redirect::default())
//!     .build()
//!     .then_send(Event::ReceiveResponse);
//! ```

use crate::middleware::{Middleware, Next, Request};
use crate::{Client, RawResponse, Result};
use http::StatusCode;
use url::ParseError;

// List of acceptable 300-series redirect codes.
const REDIRECT_CODES: &[StatusCode] = &[
    StatusCode::MOVED_PERMANENTLY,
    StatusCode::FOUND,
    StatusCode::SEE_OTHER,
    StatusCode::TEMPORARY_REDIRECT,
    StatusCode::PERMANENT_REDIRECT,
];

/// A middleware which attempts to follow HTTP redirects.
#[derive(Debug)]
pub struct Redirect {
    attempts: u8,
}

impl Redirect {
    /// Create a new instance of the Redirect middleware.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crux_core::macros::effect;
    /// # use crux_http::HttpRequest;
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # #[effect]
    /// # #[allow(unused)]
    /// # enum Effect { Http(HttpRequest) }
    /// # type Http = crux_http::Http<Effect, Event>;
    ///
    /// Http::get("https://httpbin.org/redirect/2")
    ///     .middleware(crux_http::middleware::Redirect::default())
    ///     .build()
    ///     .then_send(Event::ReceiveResponse);
    /// ```
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(attempts: u8) -> Self {
        Self { attempts }
    }
}

#[async_trait::async_trait]
impl Middleware for Redirect {
    async fn handle(
        &self,
        mut request: Request,
        client: Client,
        next: Next<'_>,
    ) -> Result<RawResponse> {
        let mut redirect_count: u8 = 0;
        let mut base_url = request.url().clone();

        while redirect_count < self.attempts {
            redirect_count += 1;
            let r: Request = request.clone();
            let res: RawResponse = client.send(r).await?;
            if REDIRECT_CODES.contains(&res.status()) {
                if let Some(location) = res.header(http::header::LOCATION) {
                    let location_str = location.to_str().unwrap_or("");
                    *request.url_mut() = match url::Url::parse(location_str) {
                        Ok(valid_url) => {
                            base_url = valid_url;
                            base_url.clone()
                        }
                        Err(ParseError::RelativeUrlWithoutBase) => base_url.join(location_str)?,
                        Err(e) => return Err(e.into()),
                    };
                }
            } else {
                break;
            }
        }

        Ok(next.run(request, client).await?)
    }
}

impl Default for Redirect {
    fn default() -> Self {
        Self { attempts: 3 }
    }
}
