use alloc::{
    format,
    string::{String, ToString},
};
use core::time::Duration;

use chrono::{DateTime, Utc, serde::ts_milliseconds_option::deserialize as ts_milliseconds_option};
use crux_core::{
    App, Command,
    macros::effect,
    render::{RenderOperation, render},
};
use crux_http::{command::Http, protocol::HttpRequest};
use crux_time::{clock::Time, operation as time};
use facet::Facet;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::sse::{ServerSentEvents, SseRequest};

const API_URL: &str = "https://crux-counter.fly.dev";

/// The first wait before reopening a stream that ended, and the longest.
const WATCH_BACKOFF_MIN: Duration = Duration::from_secs(1);
const WATCH_BACKOFF_MAX: Duration = Duration::from_secs(30);

// ANCHOR: model
#[derive(Serialize)]
pub struct Model {
    count: Count,
    /// Why the last request failed, until the next update arrives.
    #[serde(skip)]
    error: Option<String>,
    /// A Server-Sent Events stream is open (or being opened).
    #[serde(skip)]
    watching: bool,
    /// Identifies the current stream, so a stale end or retry is ignored.
    #[serde(skip)]
    watch: u32,
    /// How long to wait before reopening a stream that ended.
    #[serde(skip)]
    backoff: Duration,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Count {
    value: isize,
    #[serde(deserialize_with = "ts_milliseconds_option")]
    updated_at: Option<DateTime<Utc>>,
}
// ANCHOR_END: model

impl Default for Model {
    fn default() -> Self {
        Self {
            count: Count::default(),
            error: None,
            watching: false,
            watch: 0,
            backoff: WATCH_BACKOFF_MIN,
        }
    }
}

#[derive(Facet, Serialize, Deserialize, Debug, Clone, Default)]
pub struct ViewModel {
    pub text: String,
    pub confirmed: bool,
    /// The count as a number, for shells that draw it rather than print it.
    pub value: i32,
    /// Why the last request failed, until the next update arrives.
    pub error: Option<String>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Event {
    // events from the shell
    Get,
    Increment,
    Decrement,
    StartWatch,

    // events local to the core
    #[serde(skip)]
    #[facet(skip)]
    Set(#[facet(opaque)] crux_http::Result<crux_http::Response<Count>>),
    #[serde(skip)]
    #[facet(skip)]
    Update(#[facet(opaque)] Count),
    /// The Server-Sent Events stream with this id ended.
    #[serde(skip)]
    #[facet(skip)]
    WatchEnded(u32),
    /// The wait after stream `id` ended is over.
    #[serde(skip)]
    #[facet(skip)]
    Rewatch(u32),
}

// ANCHOR: effect
#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
    ServerSentEvents(SseRequest),
    TimeNotifyAfter(time::NotifyAfter),
    TimeClear(time::ClearTimer),
}
// ANCHOR_END: effect

#[derive(Default)]
pub struct Counter;

impl App for Counter {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::Get => Http::get(API_URL)
                .expect_json()
                .build()
                .then_send(Event::Set),
            Event::Set(Ok(mut response)) => {
                let count = response.take_body().unwrap();
                Command::event(Event::Update(count))
            }
            Event::Set(Err(e)) => {
                model.error = Some(e.to_string());
                render()
            }
            Event::Update(count) => {
                model.count = count;
                model.error = None;
                // the server is reachable again
                model.backoff = WATCH_BACKOFF_MIN;
                render()
            }
            // ...
            Event::Increment => {
                // optimistic update
                model.count = Count {
                    value: model.count.value + 1,
                    updated_at: None,
                };

                let call_api = {
                    let base = Url::parse(API_URL).unwrap();
                    let url = base.join("/inc").unwrap();
                    Http::post(url).expect_json().build().then_send(Event::Set)
                };

                render().and(call_api)
            }
            Event::Decrement => {
                // optimistic update
                model.count = Count {
                    value: model.count.value - 1,
                    updated_at: None,
                };

                let call_api = {
                    let base = Url::parse(API_URL).unwrap();
                    let url = base.join("/dec").unwrap();
                    Http::post(url).expect_json().build().then_send(Event::Set)
                };

                render().and(call_api)
            }
            Event::StartWatch => watch(model),
            Event::WatchEnded(id) => {
                if id != model.watch {
                    return Command::done();
                }
                // the stream ended: reopen it after a wait that doubles each time
                model.watching = false;
                let wait = model.backoff;
                model.backoff = (wait * 2).min(WATCH_BACKOFF_MAX);
                let (notify, _handle) = Time::notify_after(wait);
                notify.then_send(move |_| Event::Rewatch(id))
            }
            Event::Rewatch(id) => {
                if id == model.watch {
                    watch(model)
                } else {
                    Command::done()
                }
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        let suffix = model
            .count
            .updated_at
            .map_or_else(|| " (pending)".to_string(), |d| format!(" ({d})"));

        let value = model.count.value;
        Self::ViewModel {
            text: value.to_string() + &suffix,
            confirmed: model.count.updated_at.is_some(),
            value: i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX }),
            error: model.error.clone(),
        }
    }
}

/// Open the Server-Sent Events stream, and say when it ends, so that it can
/// be reopened. A stream that is already open is left alone.
fn watch(model: &mut Model) -> Command<Effect, Event> {
    if model.watching {
        return Command::done();
    }
    model.watching = true;
    // a new id, so the end of an older stream, or a pending retry, is ignored
    model.watch = model.watch.wrapping_add(1);
    let id = model.watch;

    let base = Url::parse(API_URL).unwrap();
    let url = base.join("/sse").unwrap();
    Command::new(move |ctx| async move {
        let mut updates = ServerSentEvents::get(url).into_stream(ctx.clone());
        while let Some(count) = updates.next().await {
            ctx.send_event(Event::Update(count));
        }
        ctx.send_event(Event::WatchEnded(id));
    })
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use crux_core::{App as _, Command};
    use crux_http::{
        HttpError,
        protocol::{HttpRequest, HttpResponse, HttpResult},
        testing::ResponseBuilder,
    };

    use super::{
        Counter, Effect, EffectTestExt, Event, Model, WATCH_BACKOFF_MAX, WATCH_BACKOFF_MIN,
    };
    use crate::{
        Count,
        sse::{SseRequest, SseResponse},
    };

    // ANCHOR: simple_tests
    /// Test that a `Get` event causes the app to fetch the current
    /// counter value from the web API
    #[test]
    fn get_counter() {
        let app = Counter;
        let mut model = Model::default();

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
                HttpResult::Ok(
                    HttpResponse::ok()
                        .body(r#"{ "value": 1, "updated_at": 1672531200000 }"#)
                        .build(),
                )
            })
            .expect_event();
        let expected = Event::Set(Ok(ResponseBuilder::ok()
            .body(Count {
                value: 1,
                updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
            })
            .build()));
        assert_eq!(actual, expected);

        // send the `Set` event back to the app
        let mut cmd = app.update(actual, &mut model);

        // check in flight that the app has not been updated with the server data
        let view = app.view(&model);
        assert_eq!(view.text, "0 (pending)");

        // this should generate an `Update` event
        let event = cmd.expect_one_event();
        assert_eq!(
            event,
            Event::Update(Count {
                value: 1,
                updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
            })
        );

        // send the `Update` event back to the app
        let mut cmd = app.update(event, &mut model);

        // the model should be updated
        assert_eq!(
            model.count,
            Count {
                value: 1,
                updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
            }
        );

        // the app should ask the shell to render
        cmd.expect_only_render();

        // the view should be updated
        let view = app.view(&model);
        assert_eq!(view.text, "1 (2023-01-01 00:00:00 UTC)");
        assert!(view.confirmed);
    }
    // ANCHOR_END: simple_tests

    // Test that an `Increment` event causes the app to increment the counter
    #[test]
    fn increment_counter() {
        let app = Counter;

        // set up our initial model as though we've previously fetched the counter
        let mut model = Model {
            count: Count {
                value: 1,
                updated_at: Some(Utc.with_ymd_and_hms(2022, 12, 31, 23, 59, 0).unwrap()),
            },
            ..Model::default()
        };

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
                HttpResult::Ok(
                    HttpResponse::ok()
                        .body(r#"{ "value": 2, "updated_at": 1672531200000 }"#)
                        .build(),
                )
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

        // send the `Set` event back to the app
        let mut cmd = app.update(event, &mut model);

        // this should generate an `Update` event
        let event = cmd.expect_one_event();
        assert_eq!(
            event,
            Event::Update(Count {
                value: 2,
                updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
            })
        );

        // send the `Update` event back to the app — render fires
        app.update(event, &mut model).expect_only_render();

        // the model should be updated
        insta::assert_yaml_snapshot!(model, @r#"
        count:
          value: 2
          updated_at: "2023-01-01T00:00:00Z"
        "#);
    }

    /// Test that a `Decrement` event causes the app to decrement the counter
    #[test]
    fn decrement_counter() {
        let app = Counter;

        // set up our initial model as though we've previously fetched the counter
        let mut model = Model {
            count: Count {
                value: 0,
                updated_at: Some(Utc.with_ymd_and_hms(2022, 12, 31, 23, 59, 0).unwrap()),
            },
            ..Model::default()
        };

        // we are expecting our model to be updated "optimistically" before the
        // HTTP request completes, so the value should have been updated
        // but not the timestamp
        let event = app
            .update(Event::Decrement, &mut model)
            // the app should ask the shell to render the optimistic update
            .expect_render()
            // and send an HTTP post; resolve it and pull the `Set` event
            .resolve_http(|op| {
                assert_eq!(
                    op,
                    &HttpRequest::post("https://crux-counter.fly.dev/dec").build()
                );
                HttpResult::Ok(
                    HttpResponse::ok()
                        .body(r#"{ "value": -1, "updated_at": 1672531200000 }"#)
                        .build(),
                )
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
                updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
            })
        );

        // send the `Update` event back to the app — render fires
        app.update(event, &mut model).expect_only_render();

        // the model should be updated
        insta::assert_yaml_snapshot!(model, @r#"
        count:
          value: -1
          updated_at: "2023-01-01T00:00:00Z"
        "#);
    }

    #[test]
    fn server_sent_events() {
        let app = Counter;
        let mut model = Model::default();

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
                b"data: {\"value\":1,\"updated_at\":1672531200000}\n\n".to_vec(),
            ))
            .unwrap();

        // the app should emit an `Update` event with the new `Count`
        let event = cmd.expect_one_event();
        assert_eq!(
            event,
            Event::Update(Count {
                value: 1,
                updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
            })
        );

        // we can resolve the request with another simulated response
        request
            .resolve(SseResponse::Chunk(
                b"data: {\"value\":2,\"updated_at\":1672531200000}\n\n".to_vec(),
            ))
            .unwrap();

        // the app should emit another `Update` event with the new `Count`
        let event = cmd.expect_one_event();
        assert_eq!(
            event,
            Event::Update(Count {
                value: 2,
                updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
            })
        );
    }

    /// An event split across two chunks still arrives, once. When the stream
    /// ends (`Done`), the command says so, then finishes.
    #[test]
    fn server_sent_events_split_across_chunks() {
        let app = Counter;
        let mut model = Model::default();
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
                updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
            })
        );

        request.resolve(SseResponse::Done).unwrap();
        assert_eq!(cmd.expect_one_event(), Event::WatchEnded(model.watch));
        assert!(cmd.is_done());
    }

    /// A failed request shows an error, rather than crashing the app, until
    /// the next update from the server.
    #[test]
    fn a_failed_request_shows_an_error_until_the_next_update() {
        let app = Counter;
        let mut model = Model::default();

        app.update(
            Event::Set(Err(HttpError::Io("network down".to_string()))),
            &mut model,
        )
        .expect_only_render();
        assert_eq!(
            app.view(&model).error.as_deref(),
            Some("IO error: network down")
        );

        let _ = app.update(
            Event::Update(Count {
                value: 4,
                updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
            }),
            &mut model,
        );
        let view = app.view(&model);
        assert_eq!(view.error, None);
        assert_eq!(view.value, 4);
    }

    /// A stream that ends is reopened after a wait, which doubles each time up
    /// to a limit, and starts again from the minimum once data arrives.
    #[test]
    fn a_stream_that_ends_is_reopened_after_a_backoff() {
        let app = Counter;
        let mut model = Model::default();

        let mut cmd = app.update(Event::StartWatch, &mut model);
        let mut stream = cmd.expect_one_effect().expect_server_sent_events();

        let mut expected = WATCH_BACKOFF_MIN;
        for _ in 0..8 {
            // the stream ends: wait, then reopen
            stream.resolve(SseResponse::Done).unwrap();
            let ended = cmd.expect_one_event();
            let mut waiting = app.update(ended, &mut model);
            let mut timer = waiting.expect_one_effect().expect_time_notify_after();
            assert_eq!(timer.operation.duration, expected.into());
            expected = (expected * 2).min(WATCH_BACKOFF_MAX);

            let id = timer.operation.id;
            timer.resolve(id).unwrap();
            let rewatch = waiting.expect_one_event();
            cmd = app.update(rewatch, &mut model);
            stream = cmd.expect_one_effect().expect_server_sent_events();

            // it is open again, so another `StartWatch` must not open a second one
            assert!(no_effects(&mut app.update(Event::StartWatch, &mut model)));
        }
        assert_eq!(model.backoff, WATCH_BACKOFF_MAX);

        // data from the server resets the wait
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
                .expect_time_notify_after()
                .operation
                .duration,
            WATCH_BACKOFF_MIN.into()
        );
    }

    /// `StartWatch` during the wait opens the stream at once; when the old
    /// timer fires, and when an old stream ends, nothing else happens.
    #[test]
    fn stale_stream_ends_and_timers_are_ignored() {
        let app = Counter;
        let mut model = Model::default();

        let mut cmd = app.update(Event::StartWatch, &mut model);
        let mut stream = cmd.expect_one_effect().expect_server_sent_events();
        stream.resolve(SseResponse::Done).unwrap();
        let ended = cmd.expect_one_event();
        let mut waiting = app.update(ended, &mut model);
        let mut timer = waiting.expect_one_effect().expect_time_notify_after();

        // the shell asks for the stream again before the timer fires
        let _stream = app
            .update(Event::StartWatch, &mut model)
            .expect_one_effect()
            .expect_server_sent_events();
        assert!(model.watching);

        // the old timer fires: its `Rewatch` is for a stream that is gone
        let id = timer.operation.id;
        timer.resolve(id).unwrap();
        let stale = waiting.expect_one_event();
        assert!(no_effects(&mut app.update(stale, &mut model)));

        // and so is the end of the first stream
        assert!(no_effects(
            &mut app.update(Event::WatchEnded(1), &mut model)
        ));
        assert!(model.watching);
    }

    /// The view carries the count as a number too, for shells that draw it.
    #[test]
    fn the_view_has_the_count_as_a_number() {
        let app = Counter;
        let model = Model {
            count: Count {
                value: -3,
                updated_at: None,
            },
            ..Model::default()
        };
        let view = app.view(&model);
        assert_eq!(view.value, -3);
        assert_eq!(view.text, "-3 (pending)");
        assert!(!view.confirmed);
    }

    fn no_effects(cmd: &mut Command<Effect, Event>) -> bool {
        cmd.effects().next().is_none()
    }
}
