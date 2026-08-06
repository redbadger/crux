//! What a feature actually receives when the server rejects a request.
//!
//! Every test here resolves a real `Command` with a real [`HttpResult`], because that is
//! the only way to observe the value an app is given. A test that instead *fabricates* the
//! value — `Ok(ResponseBuilder::with_status(409).body(…).build())` — asserts a state the
//! machinery below can never produce, and so passes while the code it covers is dead.
//! `ResponseBuilder` now refuses to build such a response; [`rejection`] builds the real
//! thing.
//!
//! [`rejection`]: crux_http::testing::rejection

mod shared {
    use crux_core::{Command, macros::effect};
    use crux_http::{command::Http, protocol::HttpRequest};
    use serde::{Deserialize, Serialize};

    /// The body the API returns on success.
    #[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
    pub struct Leave {
        pub id: String,
    }

    #[derive(Debug)]
    pub enum Event {
        Booked(crux_http::Result<crux_http::Response<Leave>>),
    }

    #[effect]
    pub enum Effect {
        Http(HttpRequest),
    }

    pub const URL: &str = "https://example.com/leave";

    /// A body-less write, of the kind whose only interesting outcome is the rejection.
    pub fn book() -> Command<Effect, Event> {
        Http::post(URL)
            .expect_json::<Leave>()
            .build()
            .then_send(Event::Booked)
    }

    /// What such a feature wants to show the user: the server's sentence if it sent one,
    /// and only otherwise the status.
    pub fn message(result: crux_http::Result<crux_http::Response<Leave>>) -> String {
        match result {
            // No status check here: this arm cannot see a 4xx or 5xx.
            Ok(response) => format!("booked {}", response.body().unwrap().id),
            Err(error) => error
                .body_json::<serde_json::Value>()
                .ok()
                .and_then(|body| body["error"].as_str().map(str::to_string))
                .unwrap_or_else(|| error.to_string()),
        }
    }
}

use crux_http::{
    protocol::{HttpResponse, HttpResult},
    testing::{rejection, rejection_from},
};
use shared::{Effect, Event, Leave, URL, book, message};

/// Drive `book()` to completion against the given protocol response, returning what the
/// app's `Booked` event is handed.
fn resolve(response: HttpResponse) -> crux_http::Result<crux_http::Response<Leave>> {
    let mut cmd = book();

    let Effect::Http(mut request) = cmd.effects().next().expect("an HTTP effect");
    assert_eq!(request.operation.url, URL);

    request
        .resolve(HttpResult::Ok(response))
        .expect("should resolve");

    let Event::Booked(result) = cmd.events().next().expect("an event");
    result
}

#[test]
fn a_rejection_arrives_as_an_error_carrying_the_servers_message() {
    let result = resolve(
        HttpResponse::status(409)
            .header("content-type", "application/json")
            .json(serde_json::json!({"error": "those dates overlap a booked day"}))
            .build(),
    );

    // The status is not observable as a response — only as an error.
    let error = result.expect_err("a 409 is never Ok");
    assert_eq!(error.code(), Some(409));
    assert_eq!(
        message(Err(error)),
        "those dates overlap a booked day",
        "the server's own message must survive, not just the status"
    );
}

/// `expect_json::<Leave>()` cannot decode an error envelope — and doesn't try. Decoding is
/// skipped for an error status, so the raw body reaches the app whatever it contains.
#[test]
fn the_error_body_survives_a_typed_expectation() {
    let error = resolve(
        HttpResponse::status(422)
            .json(serde_json::json!({"error": "end date is before start date"}))
            .build(),
    )
    .expect_err("a 422 is never Ok");

    assert_eq!(
        error.body(),
        Some(&br#"{"error":"end date is before start date"}"#[..])
    );
}

/// `Retry-After` is only in the headers — not in the status, not in the body — so this is
/// the case that proves the headers have to survive the conversion into an error.
#[test]
fn a_rejections_headers_reach_the_app() {
    let error = resolve(
        HttpResponse::status(503)
            .header("retry-after", "120")
            .header("content-type", "text/plain")
            .body(b"maintenance until 09:00".to_vec())
            .build(),
    )
    .expect_err("a 503 is never Ok");

    assert_eq!(error.header("Retry-After").unwrap(), "120");
    assert_eq!(
        error
            .content_type()
            .map(|mime| mime.essence_str().to_string()),
        Some("text/plain".to_string())
    );
    assert_eq!(error.body(), Some(&b"maintenance until 09:00"[..]));
}

/// The value `testing::rejection` builds is the value the machinery delivers — so a test
/// that asserts against it is testing the live path. With headers in play, that equality
/// only holds for the header-carrying form.
#[test]
fn the_testing_helper_matches_what_the_shell_produces() {
    let body = r#"{"error":"that would create a management cycle"}"#;

    let from_shell = resolve(HttpResponse::status(409).body(body).build());
    assert_eq!(from_shell, rejection::<Leave>(409, body));

    let with_headers = resolve(
        HttpResponse::status(409)
            .header("content-type", "application/json")
            .body(body)
            .build(),
    );
    assert_eq!(
        with_headers,
        rejection_from::<Leave>(
            HttpResponse::status(409)
                .header("content-type", "application/json")
                .body(body)
                .build()
        )
    );
}

#[test]
fn a_success_still_decodes_the_body() {
    let leave = Leave {
        id: "leave-1".to_string(),
    };
    let result = resolve(HttpResponse::status(201).json(&leave).build());

    assert_eq!(message(result), "booked leave-1");
}
