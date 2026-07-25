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

        let mut new_req = Self::new(method, url);

        for (name, values) in &req {
            // Convert to HeaderName to satisfy the IntoHeaderName 'static bound on &str.
            if let Ok(hn) = http::HeaderName::from_bytes(name.as_str().as_bytes()) {
                for value in values {
                    if let Ok(hv) = HeaderValue::from_str(value.as_str()) {
                        new_req.append_header(hn.clone(), hv);
                    }
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
        let mut ht_req = Self::new(method, req.url().clone());

        for (name, value) in &req {
            ht_req.append_header(name.as_str(), value.to_str().unwrap_or(""));
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
    /// **The body is always empty.** `http_types::Body::into_bytes()` is async and cannot
    /// be called from a `From` impl. If you need the response body, read it from the
    /// `http_types::Response` before converting, then set it on the result with
    /// `body_bytes` or by constructing `RawResponse` directly.
    fn from(res: http_types::Response) -> Self {
        let status = http::StatusCode::from_u16(u16::from(res.status()))
            .unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR);

        let mut headers = http::HeaderMap::new();
        for (name, values) in &res {
            if let Ok(hn) = HeaderName::from_bytes(name.as_str().as_bytes()) {
                for value in values {
                    if let Ok(hv) = HeaderValue::from_str(value.as_str()) {
                        headers.append(hn.clone(), hv);
                    }
                }
            }
        }

        Self::new(status, headers, vec![])
    }
}

#[cfg(test)]
mod tests {
    use crate::{Request, Url};
    use http::{HeaderValue, Method};

    #[test]
    fn request_to_http_types_preserves_multi_value_headers() {
        let mut req = Request::new(Method::GET, Url::parse("https://example.com").unwrap());
        req.append_header(
            http::header::COOKIE,
            HeaderValue::from_static("session=abc"),
        );
        req.append_header(http::header::COOKIE, HeaderValue::from_static("pref=dark"));

        let ht: http_types::Request = req.into();

        let cookie_values: Vec<String> = ht
            .header("cookie")
            .into_iter()
            .flat_map(http_types::headers::HeaderValues::iter)
            .map(ToString::to_string)
            .collect();

        assert_eq!(cookie_values.len(), 2, "both cookie values must survive");
        assert!(cookie_values.iter().any(|v| v == "session=abc"));
        assert!(cookie_values.iter().any(|v| v == "pref=dark"));
    }

    #[test]
    fn http_types_request_to_request_preserves_multi_value_headers() {
        let mut ht = http_types::Request::new(
            http_types::Method::Get,
            "https://example.com".parse::<http_types::Url>().unwrap(),
        );
        ht.append_header("set-cookie", "a=1");
        ht.append_header("set-cookie", "b=2");

        let req: Request = ht.into();

        assert_eq!(
            req.header_all("set-cookie").into_iter().count(),
            2,
            "both set-cookie values must survive"
        );
    }
}
