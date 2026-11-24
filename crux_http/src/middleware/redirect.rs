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
use crate::prelude::*;
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
    /// Create a new instance of the Redirect middleware, which attempts to follow redirects
    /// up to `attempts` times.
    ///
    /// Consider using [`Redirect::default`] for the default of 3 redirect attempts.
    ///
    /// This middleware follows redirects from the `Location` header when the server returns
    /// any of the following status codes:
    /// - 301 Moved Permanently
    /// - 302 Found
    /// - 303 See Other
    /// - 307 Temporary Redirect
    /// - 308 Permanent Redirect
    ///
    /// # Errors
    ///
    /// Returns an error if the `Location` header value is not a valid URL, or if it contains
    /// non-ASCII bytes (e.g. a UTF-8 encoded path).
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
                    let location_str = location.to_str().map_err(|_| {
                        crate::HttpError::Io("redirect Location header is not valid ASCII".into())
                    })?;
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
    /// Create a new instance of the Redirect middleware with the default of 3 redirect attempts.
    fn default() -> Self {
        Self { attempts: 3 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Client;
    use crate::protocol::HttpResponse;
    use crate::testing::FakeShell;

    // The redirect middleware always makes one extra request via `next.run` after the
    // loop completes (whether it broke early or ran out of attempts).  Each iteration
    // inside the loop calls `client.send`, then `next.run` fires once at the end.
    // For N redirects followed by a terminal response the shell therefore receives
    // N+1 (loop) + 1 (next.run) = N+2 requests total.

    #[futures_test::test]
    async fn follows_absolute_redirect() {
        let shell = FakeShell::default();
        // Request 1 (loop iter 1): 301 → update URL
        shell.provide_response(
            HttpResponse::status(301)
                .header("location", "https://example.com/new")
                .build(),
        );
        // Request 2 (loop iter 2): 200 → break
        shell.provide_response(HttpResponse::ok().build());
        // Request 3 (next.run): the actual final response returned to the caller
        shell.provide_response(HttpResponse::ok().body("final").build());

        let client = Client::new(shell.clone()).with(Redirect::new(3));
        let mut response = client.get("https://example.com/old").await.unwrap();

        assert_eq!(response.body_string().unwrap(), "final");
        let reqs = shell.take_requests_received();
        assert_eq!(reqs.len(), 3);
        assert_eq!(reqs[0].url, "https://example.com/old");
        assert_eq!(reqs[1].url, "https://example.com/new");
        assert_eq!(reqs[2].url, "https://example.com/new");
    }

    #[futures_test::test]
    async fn follows_relative_redirect() {
        let shell = FakeShell::default();
        shell.provide_response(
            HttpResponse::status(302)
                .header("location", "/other")
                .build(),
        );
        shell.provide_response(HttpResponse::ok().build());
        shell.provide_response(HttpResponse::ok().body("done").build());

        let client = Client::new(shell.clone()).with(Redirect::new(3));
        let mut response = client.get("https://example.com/start").await.unwrap();

        assert_eq!(response.body_string().unwrap(), "done");
        let reqs = shell.take_requests_received();
        assert_eq!(reqs[1].url, "https://example.com/other");
        assert_eq!(reqs[2].url, "https://example.com/other");
    }

    #[futures_test::test]
    async fn non_ascii_location_header_returns_io_error() {
        // "é" encodes to UTF-8 bytes [0xC3, 0xA9].  Both are opaque bytes (>= 0x80)
        // that HeaderValue::from_str accepts but to_str() rejects.  Our fix maps
        // that to_str() failure to HttpError::Io rather than silently looping.
        let shell = FakeShell::default();
        shell.provide_response(
            HttpResponse::status(301)
                .header("location", "é/other")
                .build(),
        );

        let client = Client::new(shell.clone()).with(Redirect::new(3));
        let result = client.get("https://example.com/start").await;

        assert!(
            matches!(result, Err(crate::HttpError::Io(_))),
            "non-ASCII Location header must return HttpError::Io, got: {result:?}"
        );
    }

    #[futures_test::test]
    async fn follows_303_redirect() {
        let shell = FakeShell::default();
        shell.provide_response(
            HttpResponse::status(303)
                .header("location", "https://example.com/new")
                .build(),
        );
        shell.provide_response(HttpResponse::ok().build());
        shell.provide_response(HttpResponse::ok().body("303 done").build());

        let client = Client::new(shell.clone()).with(Redirect::new(3));
        let mut response = client.get("https://example.com/old").await.unwrap();

        assert_eq!(response.body_string().unwrap(), "303 done");
        assert_eq!(
            shell.take_requests_received()[1].url,
            "https://example.com/new"
        );
    }

    #[futures_test::test]
    async fn follows_307_redirect() {
        let shell = FakeShell::default();
        shell.provide_response(
            HttpResponse::status(307)
                .header("location", "https://example.com/new")
                .build(),
        );
        shell.provide_response(HttpResponse::ok().build());
        shell.provide_response(HttpResponse::ok().body("307 done").build());

        let client = Client::new(shell.clone()).with(Redirect::new(3));
        let mut response = client.get("https://example.com/old").await.unwrap();

        assert_eq!(response.body_string().unwrap(), "307 done");
        assert_eq!(
            shell.take_requests_received()[1].url,
            "https://example.com/new"
        );
    }

    #[futures_test::test]
    async fn follows_308_redirect() {
        let shell = FakeShell::default();
        shell.provide_response(
            HttpResponse::status(308)
                .header("location", "https://example.com/new")
                .build(),
        );
        shell.provide_response(HttpResponse::ok().build());
        shell.provide_response(HttpResponse::ok().body("308 done").build());

        let client = Client::new(shell.clone()).with(Redirect::new(3));
        let mut response = client.get("https://example.com/old").await.unwrap();

        assert_eq!(response.body_string().unwrap(), "308 done");
        assert_eq!(
            shell.take_requests_received()[1].url,
            "https://example.com/new"
        );
    }

    #[futures_test::test]
    async fn redirect_with_no_location_header_keeps_original_url() {
        // A 301 with no Location header: the middleware silently skips URL rewriting
        // and the loop continues, eventually falling through to next.run with the
        // original URL unchanged.
        let shell = FakeShell::default();
        shell.provide_response(HttpResponse::status(301).build()); // no Location
        shell.provide_response(HttpResponse::ok().build()); // loop iter 2 breaks
        shell.provide_response(HttpResponse::ok().body("same url").build()); // next.run

        let client = Client::new(shell.clone()).with(Redirect::new(3));
        let mut response = client.get("https://example.com/start").await.unwrap();

        assert_eq!(response.body_string().unwrap(), "same url");
        let reqs = shell.take_requests_received();
        // All three requests go to the original URL — no rewrite happened.
        assert!(reqs.iter().all(|r| r.url == "https://example.com/start"));
    }

    #[futures_test::test]
    async fn stops_after_max_attempts() {
        let shell = FakeShell::default();
        // With attempts=2: loop runs twice (both 301), then next.run fires once.
        shell.provide_response(
            HttpResponse::status(301)
                .header("location", "https://example.com/loop")
                .build(),
        );
        shell.provide_response(
            HttpResponse::status(301)
                .header("location", "https://example.com/loop")
                .build(),
        );
        shell.provide_response(HttpResponse::ok().body("gave up").build());

        let client = Client::new(shell.clone()).with(Redirect::new(2));
        let mut res = client.get("https://example.com/start").await.unwrap();

        assert_eq!(res.body_string().unwrap(), "gave up");
        assert_eq!(shell.take_requests_received().len(), 3);
    }
}
