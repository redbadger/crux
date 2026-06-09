//! Conversion impls between `crux_http` types and `http_types` types.
//!
//! Enabled only when the `http-types` feature is active.

use http::{HeaderName, HeaderValue};

use crate::{RawResponse, Request};

// ── Request ──────────────────────────────────────────────────────────────────

impl From<http_types::Request> for Request {
    /// Convert an `http_types::Request` into a `crux_http::Request`.
    ///
    /// The body is left empty because `http_types::Body::into_bytes()` is async
    /// and cannot be called here. Set the body separately after conversion if needed.
    fn from(req: http_types::Request) -> Self {
        let method = req
            .method()
            .to_string()
            .parse::<http::Method>()
            .unwrap_or(http::Method::GET);
        let url = req.url().clone();

        let mut new_req = Request::new(method, url);

        for (name, values) in req.iter() {
            // Convert to HeaderName to satisfy the IntoHeaderName 'static bound on &str.
            if let Ok(hn) = http::HeaderName::from_bytes(name.as_str().as_bytes()) {
                for value in values.iter() {
                    new_req.insert_header(hn.clone(), value.as_str());
                }
            }
        }

        new_req
    }
}

impl From<Request> for http_types::Request {
    fn from(mut req: Request) -> Self {
        let method: http_types::Method = req
            .method()
            .as_str()
            .parse()
            .unwrap_or(http_types::Method::Get);
        let mut ht_req = http_types::Request::new(method, req.url().clone());

        for (name, value) in req.iter() {
            ht_req.insert_header(name.as_str(), value.to_str().unwrap_or(""));
        }

        let bytes = req.take_body().into_bytes();
        ht_req.set_body(bytes.as_slice());
        ht_req
    }
}

// ── RawResponse ─────────────────────────────────────────────────────────────

impl From<http_types::Response> for RawResponse {
    /// Convert an `http_types::Response` into a `crux_http::RawResponse`.
    ///
    /// The body is left empty because `http_types::Body::into_bytes()` is async.
    fn from(res: http_types::Response) -> Self {
        let status = http::StatusCode::from_u16(u16::from(res.status()))
            .unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR);

        let mut headers = http::HeaderMap::new();
        for (name, values) in res.iter() {
            if let Ok(hn) = HeaderName::from_bytes(name.as_str().as_bytes()) {
                for value in values.iter() {
                    if let Ok(hv) = HeaderValue::from_str(value.as_str()) {
                        headers.insert(hn.clone(), hv);
                    }
                }
            }
        }

        Self::new(status, headers, vec![])
    }
}
