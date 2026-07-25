//! What the capability puts on the wire, asserted against the protocol builders.
//!
//! These builders are how a test names the request it expects, so the two sides
//! have to agree exactly — a difference in headers alone is both easy to introduce
//! and confusing to debug.

mod shared {
    use crux_core::{macros::effect, render::RenderOperation};
    use crux_http::protocol::HttpRequest;

    #[effect]
    pub enum Effect {
        Http(HttpRequest),
        Render(RenderOperation),
    }

    /// The command has to be bound to *some* event; nothing here reads it.
    pub enum Event {
        Done(#[expect(dead_code)] crux_http::Result<crux_http::Response<Vec<u8>>>),
    }
}

use crux_core::Command;
use crux_http::{command::Http, protocol::HttpRequest};
use shared::{Effect, Event};

const URL: &str = "https://example.com/posts";

/// The request a `POST` with a JSON body actually produces.
fn posted(body: &serde_json::Value) -> HttpRequest {
    let mut cmd: Command<Effect, Event> = Http::post(URL)
        .body_json(body)
        .expect("the body serialises")
        .build()
        .then_send(Event::Done);

    cmd.effects()
        .next()
        .expect("an HTTP request")
        .expect_http()
        .operation
}

#[test]
fn body_json_mirrors_the_request_the_capability_builds() {
    // `HttpRequestBuilder::body_json` exists so an expected value can be written
    // in one line and still match: the capability's `body_json` sets the mime, so
    // `json` alone would differ by the `content-type` header and nothing else.
    let body = serde_json::json!({ "title": "New Post", "body": "Hello!" });

    assert_eq!(
        posted(&body),
        HttpRequest::post(URL).body_json(&body).build()
    );
}

#[test]
fn json_sets_only_the_body() {
    // The protocol builders stay dumb on purpose — they must be able to express a
    // JSON body with no `content-type` — so `json` is *not* a mirror.
    let body = serde_json::json!({ "title": "New Post" });
    let expected = HttpRequest::post(URL).json(&body).build();

    assert!(expected.headers.is_empty());
    assert_eq!(
        expected.body,
        serde_json::to_vec(&body).expect("serialisable")
    );
    assert_ne!(
        posted(&body),
        expected,
        "differs by the content-type header"
    );
}
