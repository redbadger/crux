//! HTTP Redirect middleware.
//!
//! # Examples
//!
//! ```no_run
//! # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
//! # struct Capabilities { http: crux_http::Http<Event> }
//! # fn update(caps: &Capabilities) {
//!
//! caps.http
//!     .get("https://httpbin.org/redirect/2")
//!     .middleware(crux_http::middleware::Redirect::default())
//!     .send(Event::ReceiveResponse)
//! # }
//! ```

use crate::middleware::{Middleware, Next, Request};
use crate::{Client, ResponseAsync, Result};
use http_types::{headers, StatusCode, Url};

// List of acceptable 300-series redirect codes.
const REDIRECT_CODES: &[StatusCode] = &[
    StatusCode::MovedPermanently,
    StatusCode::Found,
    StatusCode::SeeOther,
    StatusCode::TemporaryRedirect,
    StatusCode::PermanentRedirect,
];

/// A middleware which attempts to follow HTTP redirects.
#[derive(Debug)]
pub struct Redirect {
    attempts: u8,
}

impl Redirect {
    /// Create a new instance of the Redirect middleware, which attempts to follow redirects
    /// up to as many times as specified.
    ///
    /// Consider using `Redirect::default()` for the default number of redirect attempts.
    ///
    /// This middleware will follow redirects from the `Location` header if the server returns
    /// any of the following http response codes:
    /// - 301 Moved Permanently
    /// - 302 Found
    /// - 303 See other
    /// - 307 Temporary Redirect
    /// - 308 Permanent Redirect
    ///
    /// # Errors
    ///
    /// An error will be passed through the middleware stack if the value of the `Location`
    /// header is not a validly parsing url.
    ///
    /// # Caveats
    ///
    /// This will presently make at least one additional HTTP request before the actual request to
    /// determine if there is a redirect that should be followed, so as to preserve any request body.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # enum Event { ReceiveResponse(crux_http::Result<crux_http::Response<Vec<u8>>>) }
    /// # struct Capabilities { http: crux_http::Http<Event> }
    /// # fn update(caps: &Capabilities) {
    ///
    /// caps.http
    ///     .get("https://httpbin.org/redirect/2")
    ///     .middleware(crux_http::middleware::Redirect::default())
    ///     .send(Event::ReceiveResponse)
    /// # }
    /// ```
    pub fn new(attempts: u8) -> Self {
        Redirect { attempts }
    }
}

#[async_trait::async_trait]
impl Middleware for Redirect {
    async fn handle(
        &self,
        mut req: Request,
        client: Client,
        next: Next<'_>,
    ) -> Result<ResponseAsync> {
        let mut redirect_count: u8 = 0;

        // Note(Jeremiah): This is not ideal.
        //
        // HttpClient is currently too limiting for efficient redirects.
        // We do not want to make unnecessary full requests, but it is
        // presently required due to how Body streams work.
        //
        // Ideally we'd have methods to send a partial request stream,
        // without sending the body, that would potnetially be able to
        // get a server status before we attempt to send the body.
        //
        // As a work around we clone the request first (without the body),
        // and try sending it until we get some status back that is not a
        // redirect.

        let mut base_url = req.url().clone();

        while redirect_count < self.attempts {
            redirect_count += 1;
            let r: Request = req.clone();
            let res: ResponseAsync = client.send(r).await?;
            if REDIRECT_CODES.contains(&res.status()) {
                if let Some(location) = res.header(headers::LOCATION) {
                    let http_req: &mut http_types::Request = req.as_mut();
                    *http_req.url_mut() = match Url::parse(location.last().as_str()) {
                        Ok(valid_url) => {
                            base_url = valid_url;
                            base_url.clone()
                        }
                        Err(e) => match e {
                            http_types::url::ParseError::RelativeUrlWithoutBase => {
                                base_url.join(location.last().as_str())?
                            }
                            e => return Err(e.into()),
                        },
                    };
                }
            } else {
                break;
            }
        }

        Ok(next.run(req, client).await?)
    }
}

impl Default for Redirect {
    /// Create a new instance of the Redirect middleware, which attempts to follow up to
    /// 3 redirects (not including the actual request).
    fn default() -> Self {
        Self { attempts: 3 }
    }
}
