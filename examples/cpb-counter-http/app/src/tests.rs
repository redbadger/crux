//! `counter_http`'s tests, ported. The flow and assertions are the same; responses are
//! built from `crux_http`'s protocol types (its `testing` helpers need std), dates are epoch
//! milliseconds, and the view is pixels.

use alloc::{string::ToString, vec::Vec};

use crux_core::App as _;
use crux_http::{
    HttpError, Response,
    protocol::{HttpRequest, HttpResponse, HttpResult},
};

use super::{
    Count, Counter, Delay, EffectTestExt, Event, Model, PIXELS, Rgb, WATCH_BACKOFF_MAX,
    WATCH_BACKOFF_MIN,
};
use cpb_protocol::{SseRequest, SseResponse};

/// 2023-01-01T00:00:00Z
const NEW_YEAR: i64 = 1_672_531_200_000;

fn ok_json(body: &str) -> HttpResult {
    HttpResult::Ok(HttpResponse::ok().body(body).build())
}

/// The `Response<Count>` `crux_http` hands the app for a 200 with this body.
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
    assert_eq!(lit(&view), [] as [Rgb; 0]);
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

    // When the stream ends (`Done`), the command says so, then finishes.
    request.resolve(SseResponse::Done).unwrap();
    assert_eq!(cmd.expect_one_event(), Event::WatchEnded(model.watch));
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

/// `counter_http` panics on a failed request; on the board that would freeze it, so the app
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

/// Open the stream from a connected model, end it, and return the event that says so.
fn start_and_end_watch(app: &Counter, model: &mut Model) -> Event {
    let mut cmd = app.update(Event::StartWatch, model);
    let mut request = cmd.expect_one_effect().expect_server_sent_events();
    request.resolve(SseResponse::Done).unwrap();
    cmd.expect_one_event()
}

/// `counter_http` never reopens a stream that ended; a browser user reloads. The board
/// resubscribes after a back-off that doubles to a limit and resets once data arrives.
#[test]
fn a_stream_that_ends_is_reopened_after_a_backoff() {
    let app = Counter;
    let mut model = connected_model(Count::default());

    let mut cmd = app.update(Event::StartWatch, &mut model);
    let mut stream = cmd.expect_one_effect().expect_server_sent_events();

    let mut expected = WATCH_BACKOFF_MIN;
    for _ in 0..8 {
        // The stream ends: wait, then reopen.
        stream.resolve(SseResponse::Done).unwrap();
        let ended = cmd.expect_one_event();
        let mut waiting = app.update(ended, &mut model);
        let mut delay = waiting.expect_one_effect().expect_delay();
        assert_eq!(delay.operation, Delay { millis: expected });
        expected = (expected * 2).min(WATCH_BACKOFF_MAX);

        delay.resolve(()).unwrap();
        let rewatch = waiting.expect_one_event();
        cmd = app.update(rewatch, &mut model);
        stream = cmd.expect_one_effect().expect_server_sent_events();

        // It is open again, so another StartWatch must not open a second one.
        assert!(effects_none(&mut app.update(Event::StartWatch, &mut model)));
    }
    assert_eq!(model.backoff, WATCH_BACKOFF_MAX);

    // Data from the server resets the back-off.
    stream
        .resolve(SseResponse::Chunk(
            b"data: {\"value\":1,\"updated_at\":1672531200000}\n\n".to_vec(),
        ))
        .unwrap();
    let update = cmd.expect_one_event();
    let _ = app.update(update, &mut model);
    stream.resolve(SseResponse::Done).unwrap();
    let ended = cmd.expect_one_event();
    assert_eq!(
        app.update(ended, &mut model)
            .expect_one_effect()
            .expect_delay()
            .operation,
        Delay {
            millis: WATCH_BACKOFF_MIN
        }
    );
}

/// After a disconnect and reconnect, the old stream's end and the old retry timer must not
/// open a second stream next to the new one.
#[test]
fn stale_stream_ends_and_timers_are_ignored() {
    let app = Counter;
    let mut model = connected_model(Count::default());

    let ended = start_and_end_watch(&app, &mut model);
    let mut cmd = app.update(ended, &mut model);
    let mut delay = cmd.expect_one_effect().expect_delay();

    // The link drops and comes back before the timer fires; connecting opens a new stream.
    let _ = app.update(Event::Disconnected, &mut model);
    let mut connected = app.update(Event::Connected, &mut model);
    connected.expect_render();
    for event in connected.events() {
        if event == Event::StartWatch {
            let _stream = app
                .update(event, &mut model)
                .expect_one_effect()
                .expect_server_sent_events();
        }
    }
    assert!(model.watching);

    // The old timer fires: its `Rewatch` is for a stream that no longer exists.
    delay.resolve(()).unwrap();
    let stale = cmd.expect_one_event();
    assert!(effects_none(&mut app.update(stale, &mut model)));
    assert!(effects_none(
        &mut app.update(Event::WatchEnded(0), &mut model)
    ));
    assert!(model.watching);
}

/// While disconnected, a stream that ends is not retried (connecting will reopen it).
#[test]
fn no_retry_while_disconnected() {
    let app = Counter;
    let mut model = connected_model(Count::default());
    let mut cmd = app.update(Event::StartWatch, &mut model);
    let mut request = cmd.expect_one_effect().expect_server_sent_events();

    let _ = app.update(Event::Disconnected, &mut model);
    request.resolve(SseResponse::Done).unwrap();
    let ended = cmd.expect_one_event();
    assert!(effects_none(&mut app.update(ended, &mut model)));
}

fn effects_none(cmd: &mut crux_core::Command<super::Effect, Event>) -> bool {
    cmd.effects().next().is_none()
}
