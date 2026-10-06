//! counter_http's tests, ported. The flow and assertions are the same; responses are
//! built from crux_http's protocol types (its `testing` helpers need std), dates are epoch
//! milliseconds, and the view is pixels.

use alloc::{string::ToString, vec::Vec};

use crux_core::App as _;
use crux_http::{
    HttpError, Response,
    protocol::{HttpRequest, HttpResponse, HttpResult},
};

use super::{Count, Counter, EffectTestExt, Event, Model, PIXELS, Rgb};
use cpb_protocol::{SseRequest, SseResponse};

/// 2023-01-01T00:00:00Z
const NEW_YEAR: i64 = 1_672_531_200_000;

fn ok_json(body: &str) -> HttpResult {
    HttpResult::Ok(HttpResponse::ok().body(body).build())
}

/// The `Response<Count>` crux_http hands the app for a 200 with this body.
fn response(count: Count) -> Response<Count> {
    Response::try_from(HttpResponse::ok().build())
        .unwrap()
        .with_body(count)
}

fn connected_model(count: Count) -> Model {
    Model {
        count,
        connected: true,
        ..Model::default()
    }
}

fn lit(view: &super::ViewModel) -> Vec<Rgb> {
    view.pixels
        .iter()
        .copied()
        .filter(|pixel| *pixel != Rgb::default())
        .collect()
}

/// Test that a `Get` event causes the app to fetch the current
/// counter value from the web API
#[test]
fn get_counter() {
    let app = Counter;
    let mut model = connected_model(Count::default());

    // send a `Get` event to the app
    let mut cmd = app.update(Event::Get, &mut model);

    // the app should emit an HTTP request to fetch the counter; resolve it
    // with a simulated response and pull the resulting `Set` event
    let actual = cmd
        .resolve_http(|op| {
            assert_eq!(
                op,
                &HttpRequest::get("https://crux-counter.fly.dev/").build()
            );
            ok_json(r#"{ "value": 1, "updated_at": 1672531200000 }"#)
        })
        .expect_event();
    let expected = Event::Set(Ok(response(Count {
        value: 1,
        updated_at: Some(NEW_YEAR),
    })));
    assert_eq!(actual, expected);

    // send the `Set` event back to the app
    let mut cmd = app.update(actual, &mut model);

    // check in flight that the app has not been updated with the server data
    // (counter_http: "0 (pending)"): nothing lit, D13 on
    let view = app.view(&model);
    assert!(lit(&view).is_empty());
    assert!(view.led_on);

    // this should generate an `Update` event
    let event = cmd.expect_one_event();
    assert_eq!(
        event,
        Event::Update(Count {
            value: 1,
            updated_at: Some(NEW_YEAR),
        })
    );

    // send the `Update` event back to the app
    let mut cmd = app.update(event, &mut model);

    // the model should be updated
    assert_eq!(
        model.count,
        Count {
            value: 1,
            updated_at: Some(NEW_YEAR),
        }
    );

    // the app should ask the shell to render
    cmd.expect_only_render();

    // the view should be updated: one green pixel, confirmed (D13 off)
    let view = app.view(&model);
    assert_eq!(lit(&view), [Rgb { r: 0, g: 6, b: 0 }]);
    assert!(!view.led_on);
}

// Test that an `Increment` event causes the app to increment the counter
#[test]
fn increment_counter() {
    let app = Counter;

    // set up our initial model as though we've previously fetched the counter
    let mut model = connected_model(Count {
        value: 1,
        updated_at: Some(NEW_YEAR - 60_000),
    });

    // we are expecting our model to be updated "optimistically" before the
    // HTTP request completes, so the value should have been updated
    // but not the timestamp
    let event = app
        .update(Event::Increment, &mut model)
        // the app should ask the shell to render the optimistic update
        .expect_render()
        // and send an HTTP post; resolve it and pull the `Set` event
        .resolve_http(|op| {
            assert_eq!(
                op,
                &HttpRequest::post("https://crux-counter.fly.dev/inc").build()
            );
            ok_json(r#"{ "value": 2, "updated_at": 1672531200000 }"#)
        })
        .expect_event();
    assert!(matches!(event, Event::Set(_)));
    assert_eq!(
        model.count,
        Count {
            value: 2,
            updated_at: None
        }
    );

    // pending: two dim green pixels, D13 on
    let view = app.view(&model);
    assert_eq!(lit(&view), [Rgb { r: 0, g: 2, b: 0 }; 2]);
    assert!(view.led_on);

    // send the `Set` event back to the app
    let mut cmd = app.update(event, &mut model);

    // this should generate an `Update` event
    let event = cmd.expect_one_event();
    assert_eq!(
        event,
        Event::Update(Count {
            value: 2,
            updated_at: Some(NEW_YEAR),
        })
    );

    // send the `Update` event back to the app — render fires
    app.update(event, &mut model).expect_only_render();

    // the model should be updated
    assert_eq!(
        model.count,
        Count {
            value: 2,
            updated_at: Some(NEW_YEAR)
        }
    );
}

/// Test that a `Decrement` event causes the app to decrement the counter
#[test]
fn decrement_counter() {
    let app = Counter;

    // set up our initial model as though we've previously fetched the counter
    let mut model = connected_model(Count {
        value: 0,
        updated_at: Some(NEW_YEAR - 60_000),
    });

    // optimistic update, then the POST
    let event = app
        .update(Event::Decrement, &mut model)
        .expect_render()
        .resolve_http(|op| {
            assert_eq!(
                op,
                &HttpRequest::post("https://crux-counter.fly.dev/dec").build()
            );
            ok_json(r#"{ "value": -1, "updated_at": 1672531200000 }"#)
        })
        .expect_event();
    assert!(matches!(event, Event::Set(_)));
    assert_eq!(
        model.count,
        Count {
            value: -1,
            updated_at: None
        }
    );

    // send the `Set` event back to the app
    let mut update = app.update(event, &mut model);

    // this should generate an `Update` event
    let event = update.expect_one_event();
    assert_eq!(
        event,
        Event::Update(Count {
            value: -1,
            updated_at: Some(NEW_YEAR),
        })
    );

    // send the `Update` event back to the app — render fires
    app.update(event, &mut model).expect_only_render();

    // the model should be updated: one red pixel
    assert_eq!(
        model.count,
        Count {
            value: -1,
            updated_at: Some(NEW_YEAR)
        }
    );
    assert_eq!(lit(&app.view(&model)), [Rgb { r: 6, g: 0, b: 0 }]);
}

#[test]
fn server_sent_events() {
    let app = Counter;
    let mut model = connected_model(Count::default());

    // start a SSE subscription to watch for updates from the server
    let mut cmd = app.update(Event::StartWatch, &mut model);

    // the app should request a Server-Sent Events stream
    let mut request = cmd.expect_one_effect().expect_server_sent_events();
    assert_eq!(
        request.operation,
        SseRequest {
            url: "https://crux-counter.fly.dev/sse".to_string(),
        }
    );

    // resolve the request with a simulated response from the web API
    request
        .resolve(SseResponse::Chunk(
            br#"data: {"value":1,"updated_at":1672531200000}

"#
            .to_vec(),
        ))
        .unwrap();

    // the app should emit an `Update` event with the new `Count`
    let event = cmd.expect_one_event();
    assert_eq!(
        event,
        Event::Update(Count {
            value: 1,
            updated_at: Some(NEW_YEAR),
        })
    );

    // we can resolve the request with another simulated response
    request
        .resolve(SseResponse::Chunk(
            br#"data: {"value":2,"updated_at":1672531200000}

"#
            .to_vec(),
        ))
        .unwrap();

    // the app should emit another `Update` event with the new `Count`
    let event = cmd.expect_one_event();
    assert_eq!(
        event,
        Event::Update(Count {
            value: 2,
            updated_at: Some(NEW_YEAR),
        })
    );
}

// The tests below are new: things counter_http does not have.

/// BLE may split an SSE message between notifications, and the gateway forwards fetch's
/// chunks as they come; an event split across two chunks still arrives once.
#[test]
fn server_sent_events_split_across_chunks() {
    let app = Counter;
    let mut model = connected_model(Count::default());
    let mut cmd = app.update(Event::StartWatch, &mut model);
    let mut request = cmd.expect_one_effect().expect_server_sent_events();

    request
        .resolve(SseResponse::Chunk(b"data: {\"value\":5,\"upd".to_vec()))
        .unwrap();
    cmd.expect_no_events();
    request
        .resolve(SseResponse::Chunk(b"ated_at\":1672531200000}\n\n".to_vec()))
        .unwrap();
    assert_eq!(
        cmd.expect_one_event(),
        Event::Update(Count {
            value: 5,
            updated_at: Some(NEW_YEAR),
        })
    );

    // When the link drops, the shell ends the stream with `Done`, and the command finishes.
    request.resolve(SseResponse::Done).unwrap();
    assert!(cmd.is_done());
}

#[test]
fn connecting_fetches_and_watches() {
    let app = Counter;
    let mut model = Model::default();

    // Before the gateway connects: one dim blue pixel.
    assert_eq!(lit(&app.view(&model)), [Rgb { r: 0, g: 0, b: 6 }]);

    let mut cmd = app.update(Event::Connected, &mut model);
    cmd.expect_render();
    let events: Vec<Event> = cmd.events().collect();
    assert_eq!(events, [Event::Get, Event::StartWatch]);

    let _ = app.update(Event::Disconnected, &mut model);
    assert_eq!(lit(&app.view(&model)), [Rgb { r: 0, g: 0, b: 6 }]);

    // A request failed by the disconnect must not show as an error after reconnecting.
    let _ = app.update(
        Event::Set(Err(HttpError::Io("ble disconnected".to_string()))),
        &mut model,
    );
    let _ = app.update(Event::Connected, &mut model);
    assert!(lit(&app.view(&model)).is_empty(), "no error pattern");
}

/// counter_http panics on a failed request; on the board that would freeze it, so the app
/// shows an error pattern until the next good answer.
#[test]
fn a_failed_request_shows_an_error_until_the_next_success() {
    let app = Counter;
    let mut model = connected_model(Count {
        value: 3,
        updated_at: Some(NEW_YEAR),
    });

    app.update(
        Event::Set(Err(HttpError::Io("ble disconnected".to_string()))),
        &mut model,
    )
    .expect_only_render();
    let view = app.view(&model);
    assert_eq!(view.pixels[0], Rgb { r: 6, g: 0, b: 0 });
    assert_eq!(view.pixels[1], Rgb { r: 0, g: 0, b: 6 });
    assert_eq!(lit(&view).len(), PIXELS);

    let _ = app.update(
        Event::Update(Count {
            value: 4,
            updated_at: Some(NEW_YEAR),
        }),
        &mut model,
    );
    assert_eq!(lit(&app.view(&model)), [Rgb { r: 0, g: 6, b: 0 }; 4]);
}

#[test]
fn counts_beyond_ten_light_every_pixel_and_brightness_follows_the_switch() {
    let app = Counter;
    let mut model = connected_model(Count {
        value: -25,
        updated_at: Some(NEW_YEAR),
    });
    let _ = app.update(Event::Switch(true), &mut model);
    assert_eq!(lit(&app.view(&model)), [Rgb { r: 40, g: 0, b: 0 }; PIXELS]);
}
