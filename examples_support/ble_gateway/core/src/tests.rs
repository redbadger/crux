use ble_protocol::{
    GATEWAY_CHUNK, Reassembler, SseRequest, ToDevice, ToGateway, chunks, decode, encode,
};
use crux_core::{App as _, Command};
use crux_http::{
    HttpError,
    protocol::{HttpRequest, HttpResponse, HttpResult},
};

use super::{BleEvent, Effect, Event, Gateway, Link, Model, SseEvent, SseStream};

/// Every effect but `Render`.
fn effects(cmd: &mut Command<Effect, Event>) -> Vec<Effect> {
    cmd.effects().filter(|effect| !effect.is_render()).collect()
}

fn connected() -> Model {
    let app = Gateway;
    let mut model = Model::default();
    let mut cmd = app.update(Event::Connect, &mut model);
    let request = effects(&mut cmd).remove(0).expect_ble_connect();
    assert_eq!(request.operation.service, ble_protocol::SERVICE_UUID);
    assert_eq!(model.link, Link::Connecting);

    let _ = app.update(
        Event::Ble(BleEvent::Connected {
            name: "CPB Counter".to_string(),
        }),
        &mut model,
    );
    model
}

/// The device's message, as the notifications that would carry it (at 20 bytes each,
/// as if the MTU were the minimum).
fn notifications(message: &ToGateway) -> Vec<Event> {
    chunks(&encode(message), 20)
        .map(|chunk| Event::Ble(BleEvent::Data(chunk.to_vec())))
        .collect()
}

/// Decode what the gateway writes back.
fn written(data: &[u8]) -> ToDevice {
    let mut rx = Reassembler::new();
    let mut messages = rx.push(data).unwrap();
    assert_eq!(messages.len(), 1);
    decode(&messages.remove(0)).unwrap()
}

fn post_inc(id: u16) -> ToGateway {
    ToGateway::Http {
        id,
        request: HttpRequest::post("https://crux-counter.fly.dev/inc").build(),
    }
}

#[test]
fn connect_once() {
    let app = Gateway;
    let mut model = connected();
    assert!(matches!(model.link, Link::Connected { .. }));
    // A second click while connected does nothing.
    assert!(effects(&mut app.update(Event::Connect, &mut model)).is_empty());
    assert!(!app.view(&model).can_connect);
}

#[test]
fn forwards_an_http_request_and_frames_the_answer() {
    let app = Gateway;
    let mut model = connected();

    // The request arrives over several notifications; only the last one completes it.
    let mut events = notifications(&post_inc(7));
    let last = events.pop().unwrap();
    for event in events {
        assert!(effects(&mut app.update(event, &mut model)).is_empty());
    }
    let mut cmd = app.update(last, &mut model);
    let mut http = effects(&mut cmd).remove(0).expect_http();
    assert_eq!(
        http.operation,
        HttpRequest::post("https://crux-counter.fly.dev/inc").build()
    );

    let result = HttpResult::Ok(
        HttpResponse::ok()
            .body(r#"{"value":2,"updated_at":1672531200000}"#)
            .build(),
    );
    http.resolve(result.clone()).unwrap();
    let done = cmd.expect_one_event();

    let mut cmd = app.update(done, &mut model);
    let write = effects(&mut cmd).remove(0).expect_ble_write();
    assert_eq!(write.operation.chunk, GATEWAY_CHUNK);
    assert_eq!(
        written(&write.operation.data),
        ToDevice::Http { id: 7, result }
    );

    let view = app.view(&model);
    assert_eq!(view.stats.requests, 1);
    assert_eq!(view.stats.responses, 1);
    assert_eq!(view.log[0].text, "#7 ← 200 (38 B)");
}

#[test]
fn one_write_at_a_time() {
    let app = Gateway;
    let mut model = connected();

    let answer = |id| Event::HttpDone {
        id,
        result: HttpResult::Err(HttpError::Timeout),
    };

    let mut first = app.update(answer(1), &mut model);
    let mut write = effects(&mut first).remove(0).expect_ble_write();

    // While the first write is in progress, the next answer waits.
    assert!(effects(&mut app.update(answer(2), &mut model)).is_empty());
    assert!(effects(&mut app.update(answer(3), &mut model)).is_empty());

    write.resolve(Ok(())).unwrap();
    let written_event = first.expect_one_event();
    let mut next = app.update(written_event, &mut model);
    let write = effects(&mut next).remove(0).expect_ble_write();
    assert!(matches!(
        written(&write.operation.data),
        ToDevice::Http { id: 2, .. }
    ));
}

#[test]
fn forwards_an_sse_stream_until_the_server_closes_it() {
    let app = Gateway;
    let mut model = connected();
    let sse = ToGateway::Sse {
        id: 3,
        request: SseRequest {
            url: "https://crux-counter.fly.dev/sse".to_string(),
        },
    };

    let mut cmd = Command::all(
        notifications(&sse)
            .into_iter()
            .map(|event| app.update(event, &mut model)),
    );
    let mut stream = effects(&mut cmd).remove(0).expect_server_sent_events();
    assert_eq!(stream.operation.url, "https://crux-counter.fly.dev/sse");
    assert_eq!(app.view(&model).open_streams, 1);

    stream
        .resolve(SseEvent::Chunk(b"data: {\"value\":1}\n\n".to_vec()))
        .unwrap();
    let chunk = cmd.expect_one_event();
    let mut out = app.update(chunk, &mut model);
    let mut write = effects(&mut out).remove(0).expect_ble_write();
    assert_eq!(
        written(&write.operation.data),
        ToDevice::SseChunk {
            id: 3,
            data: b"data: {\"value\":1}\n\n".to_vec()
        }
    );

    // The server ends the stream while the chunk is still being written; the end waits
    // its turn.
    stream.resolve(SseEvent::Closed).unwrap();
    let done = cmd.expect_one_event();
    assert!(effects(&mut app.update(done, &mut model)).is_empty());
    write.resolve(Ok(())).unwrap();
    let mut next = app.update(out.expect_one_event(), &mut model);
    let write = effects(&mut next).remove(0).expect_ble_write();
    assert_eq!(written(&write.operation.data), ToDevice::SseDone { id: 3 });
    let view = app.view(&model);
    assert_eq!(view.open_streams, 0);
    assert_eq!(view.log[0].text, "#3 ← SSE closed by the server");
}

#[test]
fn a_failed_sse_stream_is_logged_as_a_failure_and_still_ends_on_the_device() {
    let app = Gateway;
    let mut model = connected();
    let sse = ToGateway::Sse {
        id: 5,
        request: SseRequest {
            url: "https://crux-counter.fly.dev/sse".to_string(),
        },
    };
    let mut cmd = Command::all(
        notifications(&sse)
            .into_iter()
            .map(|event| app.update(event, &mut model)),
    );
    let mut stream = effects(&mut cmd).remove(0).expect_server_sent_events();

    stream
        .resolve(SseEvent::Failed("TypeError: Failed to fetch".to_string()))
        .unwrap();
    let failed = cmd.expect_one_event();
    let mut out = app.update(failed, &mut model);
    // The device can't tell the difference: it reopens the stream either way.
    let write = effects(&mut out).remove(0).expect_ble_write();
    assert_eq!(written(&write.operation.data), ToDevice::SseDone { id: 5 });
    let view = app.view(&model);
    assert_eq!(view.open_streams, 0);
    assert_eq!(
        view.log[0].text,
        "#5 ← SSE failed: TypeError: Failed to fetch"
    );
}

#[test]
fn log_lines_are_numbered_in_order() {
    let app = Gateway;
    let mut model = connected();
    for event in notifications(&post_inc(1)) {
        let _ = app.update(event, &mut model);
    }
    let seqs: Vec<u64> = app.view(&model).log.iter().map(|line| line.seq).collect();
    // Newest first: "#1 → POST …" after "connected to …".
    assert_eq!(seqs, [1, 0]);
}

#[test]
fn disconnect_closes_streams_and_drops_late_answers() {
    let app = Gateway;
    let mut model = connected();
    let sse = ToGateway::Sse {
        id: 3,
        request: SseRequest {
            url: "https://crux-counter.fly.dev/sse".to_string(),
        },
    };
    let mut cmd = Command::all(
        notifications(&sse)
            .into_iter()
            .map(|event| app.update(event, &mut model)),
    );
    let mut stream = effects(&mut cmd).remove(0).expect_server_sent_events();

    let _ = app.update(Event::Ble(BleEvent::Disconnected), &mut model);
    assert_eq!(model.link, Link::Disconnected);
    assert_eq!(app.view(&model).open_streams, 0);
    assert_aborted(&mut cmd, &mut stream);

    // An HTTP answer that comes back after the device left is not written anywhere.
    let late = Event::HttpDone {
        id: 1,
        result: HttpResult::Err(HttpError::Timeout),
    };
    assert!(effects(&mut app.update(late, &mut model)).is_empty());
    assert!(app.view(&model).can_connect);
}

#[test]
fn a_half_frame_from_the_old_connection_is_dropped() {
    let app = Gateway;
    let mut model = connected();
    let mut events = notifications(&post_inc(1));
    events.pop();
    for event in events {
        let _ = app.update(event, &mut model);
    }
    let _ = app.update(Event::Ble(BleEvent::Disconnected), &mut model);
    let _ = app.update(
        Event::Ble(BleEvent::Connected {
            name: "CPB Counter".to_string(),
        }),
        &mut model,
    );

    let mut cmd = Command::all(
        notifications(&post_inc(2))
            .into_iter()
            .map(|event| app.update(event, &mut model)),
    );
    let http = effects(&mut cmd);
    assert_eq!(http.len(), 1);
    assert_eq!(
        http.into_iter().next().unwrap().expect_http().operation.url,
        "https://crux-counter.fly.dev/inc"
    );
}

#[test]
fn cancel_closes_a_stream() {
    let app = Gateway;
    let mut model = connected();
    let sse = ToGateway::Sse {
        id: 4,
        request: SseRequest {
            url: "https://crux-counter.fly.dev/sse".to_string(),
        },
    };
    let mut cmd = Command::all(
        notifications(&sse)
            .into_iter()
            .map(|event| app.update(event, &mut model)),
    );
    let mut stream = effects(&mut cmd).remove(0).expect_server_sent_events();

    for event in notifications(&ToGateway::Cancel { id: 4 }) {
        let _ = app.update(event, &mut model);
    }
    assert_eq!(app.view(&model).open_streams, 0);
    assert_aborted(&mut cmd, &mut stream);
}

/// Once a stream's command is aborted, what the shell still sends goes nowhere, and once
/// the command has run again (the aborted task is dropped), `resolve` fails. The shell
/// stops reading the SSE body when it does, which closes the fetch.
fn assert_aborted(cmd: &mut Command<Effect, Event>, stream: &mut crux_core::Request<SseStream>) {
    let first = stream.resolve(SseEvent::Chunk(b"data: late\n\n".to_vec()));
    assert!(cmd.events().next().is_none());
    assert!(effects(cmd).is_empty());
    let second = stream.resolve(SseEvent::Chunk(b"data: later\n\n".to_vec()));
    assert!(cmd.events().next().is_none());
    assert!(first.is_ok(), "the task is still there: {first:?}");
    assert!(second.is_err(), "the task is gone: {second:?}");
}

#[test]
fn a_response_too_large_for_the_link_becomes_an_error() {
    let app = Gateway;
    let mut model = connected();
    let huge = HttpResult::Ok(
        HttpResponse::status(502)
            .body(vec![b'x'; ble_protocol::MAX_MESSAGE])
            .build(),
    );
    let mut cmd = app.update(
        Event::HttpDone {
            id: 9,
            result: huge,
        },
        &mut model,
    );
    let write = effects(&mut cmd).remove(0).expect_ble_write();
    assert_eq!(
        written(&write.operation.data),
        ToDevice::Http {
            id: 9,
            result: HttpResult::Err(HttpError::Io("response too large for the link".to_string())),
        }
    );
}
