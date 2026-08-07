use crate::{RawResponse, Response, Result, protocol::HttpResponse};

/// Build the [`crux_http::Result`](crate::Result) a feature receives when the server
/// rejects a request.
///
/// A 4xx or 5xx response never reaches an app as `Ok(Response)` — `crux_http` converts it
/// to an [`HttpError::Http`](crate::HttpError::Http) on the `Err` side, keeping the body,
/// before the event is sent. So this, and not a
/// [`ResponseBuilder`](super::ResponseBuilder) with an error status, is the value to assert
/// against (or feed to an update function) when testing how a feature handles a rejection:
///
/// ```
/// # use crux_http::testing::rejection;
/// # #[derive(Debug, PartialEq)] enum Event { Saved(crux_http::Result<crux_http::Response<Vec<u8>>>) }
/// let event = Event::Saved(rejection(409, r#"{"error":"that overlaps a booked day"}"#));
///
/// let Event::Saved(Err(error)) = event else {
///     panic!("a 409 is delivered as an error, not a response")
/// };
/// assert_eq!(error.code(), Some(409));
/// assert_eq!(error.to_string(), "HTTP error 409: 409 Conflict");
/// ```
///
/// The `Body` type parameter is the one the app's event carries (`Vec<u8>`, `String`, or
/// whatever [`expect_json`](crate::command::RequestBuilder::expect_json) decodes to); it
/// is usually inferred, and never appears in the value, because body decoding is skipped
/// for an error status.
///
/// The status and reason phrase are produced by the same code path a real response takes,
/// so the value is byte-for-byte what the shell's response would have produced.
///
/// # Errors
///
/// Always `Err` — a rejection has no other form. The `Result` is the return type so that
/// the value can be used exactly where the app receives one.
///
/// # Panics
///
/// Panics if `status` is outside the valid HTTP range (100–999), or if it is not a client
/// (4xx) or server (5xx) error — for any other status a feature receives
/// `Ok(Response)`, which is what [`ResponseBuilder`](super::ResponseBuilder) builds.
pub fn rejection<Body>(status: u16, body: impl AsRef<[u8]>) -> Result<Response<Body>> {
    build_rejection(
        "rejection",
        HttpResponse::status(status)
            .body(body.as_ref().to_vec())
            .build(),
    )
}

/// Build the [`crux_http::Result`](crate::Result) a feature receives for a rejection, from
/// a full protocol response.
///
/// Use this over [`rejection`] when the rejection's *headers* are what the feature acts on
/// — `Retry-After`, `WWW-Authenticate`, `Content-Type` — since those are readable from the
/// error via [`HttpError::header`](crate::HttpError::header) and
/// [`HttpError::content_type`](crate::HttpError::content_type):
///
/// ```
/// # use crux_http::{protocol::HttpResponse, testing::rejection_from};
/// let result = rejection_from::<Vec<u8>>(
///     HttpResponse::status(503)
///         .header("retry-after", "120")
///         .body(b"maintenance".to_vec())
///         .build(),
/// );
///
/// let error = result.expect_err("a 503 is never Ok");
/// assert_eq!(error.header("retry-after").unwrap(), "120");
/// assert_eq!(error.body(), Some(&b"maintenance"[..]));
/// ```
///
/// It takes the same [`HttpResponse`] you would resolve a request with in an end-to-end
/// test, so the two styles of test describe a rejection the same way.
///
/// # Errors
///
/// Always `Err`, for the reason given on [`rejection`].
///
/// # Panics
///
/// Panics if the response's status is outside the valid HTTP range (100–999), or if it is
/// not a client (4xx) or server (5xx) error.
pub fn rejection_from<Body>(response: HttpResponse) -> Result<Response<Body>> {
    build_rejection("rejection_from", response)
}

/// The shared body of [`rejection`] and [`rejection_from`]. `caller` names whichever of
/// them the test actually called, so a panic points at the right function.
fn build_rejection<Body>(caller: &str, response: HttpResponse) -> Result<Response<Body>> {
    let status = response.status;
    let raw = RawResponse::try_from(response).unwrap_or_else(|_| {
        panic!("{caller} called with an out-of-range status code ({status}, must be 100–999)")
    });

    match Response::<Vec<u8>>::new(raw) {
        Err(error) => Err(error),
        Ok(_) => panic!(
            "{caller} called with status {status}, which is not a client (4xx) or server (5xx) \
             error — a feature receives that as Ok(Response), which ResponseBuilder builds"
        ),
    }
}

#[cfg(test)]
mod tests {
    use crate::{HttpError, protocol::HttpResponse};

    use super::{rejection, rejection_from};

    #[test]
    fn carries_code_reason_and_body() {
        let error = rejection::<Vec<u8>>(409, r#"{"error":"management cycle"}"#)
            .expect_err("4xx is always an error");

        let HttpError::Http {
            code,
            message,
            headers,
            body,
        } = error
        else {
            panic!("expected HttpError::Http, got {error:?}")
        };
        assert_eq!(code, 409);
        assert_eq!(message, "409 Conflict");
        assert!(headers.is_empty(), "rejection() sets no headers");
        assert_eq!(body, br#"{"error":"management cycle"}"#);
    }

    /// `rejection_from` is the header-carrying form, and must agree with `rejection` when
    /// there are no headers to carry.
    #[test]
    fn rejection_is_the_header_less_case_of_rejection_from() {
        let sugar = rejection::<Vec<u8>>(409, "nope");
        let explicit =
            rejection_from::<Vec<u8>>(HttpResponse::status(409).body(b"nope".to_vec()).build());

        assert_eq!(sugar, explicit);
    }

    #[test]
    fn rejection_from_keeps_the_headers() {
        let error = rejection_from::<Vec<u8>>(
            HttpResponse::status(401)
                .header("www-authenticate", r#"Bearer error="invalid_token""#)
                .header("content-type", "application/problem+json")
                .build(),
        )
        .expect_err("a 401 is never Ok");

        assert_eq!(
            error.header("www-authenticate").unwrap(),
            r#"Bearer error="invalid_token""#
        );
        assert_eq!(
            error.content_type().map(|mime| mime.to_string()),
            Some("application/problem+json".to_string())
        );
    }

    /// The whole point of the helper: what it produces must be indistinguishable from
    /// what the real shell → `Response::new` path produces, or tests written against it
    /// would once again be asserting a shape features never see.
    #[test]
    fn matches_the_real_conversion() {
        let from_helper =
            rejection::<String>(422, "name is required").expect_err("4xx is always an error");

        let raw = crate::RawResponse::try_from(
            HttpResponse::status(422)
                .body(b"name is required".to_vec())
                .build(),
        )
        .expect("422 is a valid status");
        let from_shell = crate::Response::<Vec<u8>>::new(raw).expect_err("4xx is always an error");

        assert_eq!(from_helper, from_shell);
    }

    #[test]
    fn body_is_optional() {
        let error = rejection::<Vec<u8>>(404, "").expect_err("4xx is always an error");
        assert_eq!(error.code(), Some(404));
        assert_eq!(error.body(), None);
    }

    #[test]
    #[should_panic(expected = "rejection called with status 200, which is not a client")]
    fn refuses_a_success_status() {
        let _ = rejection::<Vec<u8>>(200, "");
    }

    /// A panic must name the function the test called, not the one it delegates to.
    #[test]
    #[should_panic(expected = "rejection_from called with status 200, which is not a client")]
    fn rejection_from_refuses_a_success_status() {
        let _ = rejection_from::<Vec<u8>>(HttpResponse::status(200).build());
    }

    #[test]
    #[should_panic(expected = "rejection called with an out-of-range status code (99")]
    fn refuses_an_out_of_range_status() {
        let _ = rejection::<Vec<u8>>(99, "");
    }
}
