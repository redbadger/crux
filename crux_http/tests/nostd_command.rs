//! spike(no_std): the no_std subset of `command::Http`.
//!
//! Run with `cargo test -p crux_http --no-default-features --test nostd_command`.
//! The library is built without `std` (the test harness itself still has std), so
//! `Http`, `Response` and `RequestBuilder` here are the ones in `src/nostd/`.
#![cfg(not(feature = "std"))]
#![allow(clippy::redundant_pub_crate)]

mod shared {
    use crux_core::{Command, macros::effect};
    use crux_http::{HttpError, Url, command::Http, protocol::HttpRequest};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
    pub struct Count {
        pub value: isize,
    }

    #[derive(Debug, PartialEq, Eq)]
    pub enum Event {
        Get,
        Post,
        GetText,
        Chain,
        Set(crux_http::Result<crux_http::Response<Count>>),
        SetText(crux_http::Result<crux_http::Response<String>>),
        Status(u16),
    }

    #[effect]
    #[derive(Debug)]
    pub enum Effect {
        Http(HttpRequest),
    }

    pub fn update(event: Event) -> Command<Effect, Event> {
        match event {
            // The exact shape counter_http uses.
            Event::Get => Http::get("https://crux-counter.fly.dev")
                .header("Authorization", "secret-token")
                .expect_json()
                .build()
                .then_send(Event::Set),
            Event::Post => {
                let base = Url::parse("https://crux-counter.fly.dev").unwrap();
                let url = base.join("/inc").unwrap();
                Http::post(url)
                    .body_json(&Count { value: 1 })
                    .unwrap()
                    .expect_json()
                    .build()
                    .then_send(Event::Set)
            }
            Event::GetText => Http::get("https://example.com/text")
                .expect_string()
                .build()
                .then_send(Event::SetText),
            Event::Chain => Command::new(|ctx| async move {
                let mut first = Http::get("https://example.com/")
                    .build()
                    .into_future(ctx.clone())
                    .await
                    .unwrap();
                let next = first.body_string().unwrap();
                let second = Http::get(format!("https://example.com/{next}"))
                    .build()
                    .into_future(ctx.clone())
                    .await;
                let status = match second {
                    Ok(response) => response.status(),
                    Err(HttpError::Http { code, .. }) => code,
                    Err(_) => 0,
                };
                ctx.send_event(Event::Status(status));
            }),
            Event::Set(_) | Event::SetText(_) | Event::Status(_) => Command::done(),
        }
    }
}

mod tests {
    use crux_http::{
        HttpError,
        protocol::{HttpRequest, HttpResponse, HttpResult},
    };

    use crate::shared::{Count, Event, update};

    #[test]
    fn get_json_like_counter_http() {
        let mut cmd = update(Event::Get);
        let mut request = cmd.expect_one_effect().expect_http();

        // The URL is normalised and the header name lowercased, as by the std builder.
        assert_eq!(
            request.operation,
            HttpRequest::get("https://crux-counter.fly.dev/")
                .header("authorization", "secret-token")
                .build()
        );

        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .header("Content-Type", "application/json")
                    .body(r#"{ "value": 7 }"#)
                    .build(),
            ))
            .unwrap();

        let Event::Set(Ok(mut response)) = cmd.expect_one_event() else {
            panic!("expected a successful Set");
        };
        assert_eq!(response.status(), 200);
        assert_eq!(response.header("content-type"), Some("application/json"));
        assert_eq!(response.take_body(), Some(Count { value: 7 }));
    }

    #[test]
    fn post_json_sets_body_and_content_type() {
        let mut cmd = update(Event::Post);
        let request = cmd.expect_one_effect().expect_http();

        assert_eq!(
            request.operation,
            HttpRequest::post("https://crux-counter.fly.dev/inc")
                .header("content-type", "application/json")
                .body(r#"{"value":1}"#)
                .build()
        );
    }

    #[test]
    fn a_4xx_is_a_rejection_with_headers_and_body() {
        let mut cmd = update(Event::Get);
        let mut request = cmd.expect_one_effect().expect_http();

        request
            .resolve(HttpResult::Ok(
                HttpResponse::status(429)
                    .header("Retry-After", "30")
                    .body(r#"{"error":"slow down"}"#)
                    .build(),
            ))
            .unwrap();

        let Event::Set(Err(error)) = cmd.expect_one_event() else {
            panic!("a 429 is never Ok");
        };
        assert_eq!(error.code(), Some(429));
        assert_eq!(error.header("retry-after"), Some("30"));
        assert_eq!(error.body(), Some(&br#"{"error":"slow down"}"#[..]));
    }

    #[test]
    fn transport_errors_pass_through() {
        let mut cmd = update(Event::Get);
        let mut request = cmd.expect_one_effect().expect_http();

        request
            .resolve(HttpResult::Err(HttpError::Io(
                "ble disconnected".to_string(),
            )))
            .unwrap();

        assert_eq!(
            cmd.expect_one_event(),
            Event::Set(Err(HttpError::Io("ble disconnected".to_string())))
        );
    }

    #[test]
    fn invalid_status_and_bad_json() {
        let mut cmd = update(Event::Get);
        let mut request = cmd.expect_one_effect().expect_http();
        request
            .resolve(HttpResult::Ok(HttpResponse::status(42).build()))
            .unwrap();
        assert_eq!(
            cmd.expect_one_event(),
            Event::Set(Err(HttpError::InvalidStatusCode(42)))
        );

        let mut cmd = update(Event::Get);
        let mut request = cmd.expect_one_effect().expect_http();
        request
            .resolve(HttpResult::Ok(HttpResponse::ok().body("nope").build()))
            .unwrap();
        assert!(matches!(
            cmd.expect_one_event(),
            Event::Set(Err(HttpError::Json(_)))
        ));
    }

    #[test]
    fn expect_string_is_utf8_only() {
        let mut cmd = update(Event::GetText);
        let mut request = cmd.expect_one_effect().expect_http();
        request
            .resolve(HttpResult::Ok(HttpResponse::ok().body("héllo").build()))
            .unwrap();
        let Event::SetText(Ok(mut response)) = cmd.expect_one_event() else {
            panic!("expected text");
        };
        assert_eq!(response.take_body().as_deref(), Some("héllo"));

        let mut cmd = update(Event::GetText);
        let mut request = cmd.expect_one_effect().expect_http();
        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok().body(vec![0xff, 0xfe]).build(),
            ))
            .unwrap();
        assert_eq!(
            cmd.expect_one_event(),
            Event::SetText(Err(HttpError::Io(
                "could not decode body as utf-8".to_string()
            )))
        );
    }

    #[test]
    fn requests_chain_inside_a_command() {
        let mut cmd = update(Event::Chain);

        let mut first = cmd.expect_one_effect().expect_http();
        assert_eq!(
            first.operation,
            HttpRequest::get("https://example.com/").build()
        );
        first
            .resolve(HttpResult::Ok(HttpResponse::ok().body("next").build()))
            .unwrap();

        let mut second = cmd.expect_one_effect().expect_http();
        assert_eq!(
            second.operation,
            HttpRequest::get("https://example.com/next").build()
        );
        second
            .resolve(HttpResult::Ok(HttpResponse::status(404).build()))
            .unwrap();

        assert_eq!(cmd.expect_one_event(), Event::Status(404));
        assert!(cmd.is_done());
    }
}
