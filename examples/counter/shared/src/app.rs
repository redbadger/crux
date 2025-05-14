use chrono::{serde::ts_milliseconds_option::deserialize as ts_milliseconds_option, DateTime, Utc};
use crux_core::{
    macros::effect,
    render::{render, RenderOperation},
    Command,
};
use crux_http::{command::Http, protocol::HttpRequest};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::sse::{ServerSentEvents, SseRequest};

const API_URL: &str = "https://crux-counter.fly.dev";

// ANCHOR: model
#[derive(Default, Serialize)]
pub struct Model {
    count: Count,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Count {
    value: isize,
    #[serde(deserialize_with = "ts_milliseconds_option")]
    updated_at: Option<DateTime<Utc>>,
}
// ANCHOR_END: model

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ViewModel {
    pub text: String,
    pub confirmed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Event {
    // events from the shell
    Get,
    Increment,
    Decrement,
    StartWatch,

    // events local to the core
    #[serde(skip)]
    Set(crux_http::Result<crux_http::Response<Count>>),
    #[serde(skip)]
    Update(Count),
}

#[effect(typegen)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
    ServerSentEvents(SseRequest),
}

#[derive(Default)]
pub struct App;

impl crux_core::App for App {
    type Model = Model;
    type Event = Event;
    type ViewModel = ViewModel;
    type Capabilities = ();
    type Effect = Effect;

    fn update(&self, msg: Event, model: &mut Model, _caps: &()) -> Command<Effect, Event> {
        match msg {
            Event::Get => Http::get(API_URL)
                .expect_json()
                .build()
                .then_send(Event::Set),
            Event::Set(Ok(mut response)) => {
                let count = response.take_body().unwrap();
                Command::event(Event::Update(count))
            }
            Event::Set(Err(e)) => {
                panic!("Oh no something went wrong: {e:?}");
            }
            Event::Update(count) => {
                model.count = count;
                render()
            }
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
            Event::StartWatch => {
                let base = Url::parse(API_URL).unwrap();
                let url = base.join("/sse").unwrap();
                ServerSentEvents::get(url).then_send(Event::Update)
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        let suffix = match model.count.updated_at {
            None => " (pending)".to_string(),
            Some(d) => format!(" ({d})"),
        };

        Self::ViewModel {
            text: model.count.value.to_string() + &suffix,
            confirmed: model.count.updated_at.is_some(),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::{App, Event, Model};
    use crate::{capabilities::sse::SseRequest, sse::SseResponse, Count, Effect};

    use crux_core::{assert_effect, App as _};
    use crux_http::{
        protocol::{HttpRequest, HttpResponse, HttpResult},
        testing::ResponseBuilder,
    };

    // ANCHOR: simple_tests
    /// Test that a `Get` event causes the app to fetch the current
    /// counter value from the web API
    #[test]
    fn get_counter() {
        let app = App::default();
        let mut model = Model::default();

        // send a `Get` event to the app
        let mut cmd = app.update(Event::Get, &mut model, &());

        // the app should emit an HTTP request to fetch the counter
        let mut request = cmd.effects().next().unwrap().expect_http();

        // and the request should be a GET to the correct URL
        assert_eq!(
            &request.operation,
            &HttpRequest::get("https://crux-counter.fly.dev/").build()
        );

        // resolve the request with a simulated response from the web API
        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(r#"{ "value": 1, "updated_at": 1672531200000 }"#)
                    .build(),
            ))
            .unwrap();

        // the app should emit a `Set` event with the HTTP response
        let actual = cmd.events().next().unwrap();
        let expected = Event::Set(Ok(ResponseBuilder::ok()
            .body(Count {
                value: 1,
                updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
            })
            .build()));
        assert_eq!(actual, expected);

        // send the `Set` event back to the app
        let mut cmd = app.update(actual, &mut model, &());

        // check in flight that the app has not been updated with the server data
        let view = app.view(&model);
        assert_eq!(view.text, "0 (pending)");

        // this should generate an `Update` event
        let event = cmd.events().next().unwrap();
        assert_eq!(
            event,
            Event::Update(Count {
                value: 1,
                updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
            })
        );

        // send the `Update` event back to the app
        let mut cmd = app.update(event, &mut model, &());

        // the model should be updated
        assert_eq!(
            model.count,
            Count {
                value: 1,
                updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
            }
        );

        // the app should ask the shell to render
        assert_effect!(cmd, Effect::Render(_));

        // the view should be updated
        let view = app.view(&model);
        assert_eq!(view.text, "1 (2023-01-01 00:00:00 UTC)");
        assert_eq!(view.confirmed, true);
    }
    // ANCHOR_END: simple_tests

    // Test that an `Increment` event causes the app to increment the counter
    #[test]
    fn increment_counter() {
        let app = App::default();

        // set up our initial model as though we've previously fetched the counter
        let mut model = Model {
            count: Count {
                value: 1,
                updated_at: Some(Utc.with_ymd_and_hms(2022, 12, 31, 23, 59, 0).unwrap()),
            },
        };

        // send an `Increment` event to the app
        let mut cmd = app.update(Event::Increment, &mut model, &());

        // the app should ask the shell to render the optimistic update
        assert_effect!(cmd, Effect::Render(_));

        // and send an HTTP post
        let mut request = cmd.effects().next().unwrap().expect_http();
        assert_eq!(
            &request.operation,
            &HttpRequest::post("https://crux-counter.fly.dev/inc").build()
        );

        // we are expecting our model to be updated "optimistically" before the
        // HTTP request completes, so the value should have been updated
        // but not the timestamp
        assert_eq!(
            model.count,
            Count {
                value: 2,
                updated_at: None
            }
        );

        // resolve the request with a simulated response from the web API
        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(r#"{ "value": 2, "updated_at": 1672531200000 }"#)
                    .build(),
            ))
            .unwrap();

        // this should generate a `Set` event
        let event = cmd.events().next().unwrap();
        assert!(matches!(event, Event::Set(_)));

        // send the `Set` event back to the app
        let mut cmd = app.update(event, &mut model, &());

        // this should generate an `Update` event
        let event = cmd.events().next().unwrap();
        assert_eq!(
            event,
            Event::Update(Count {
                value: 2,
                updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
            })
        );

        // send the `Update` event back to the app
        let mut cmd = app.update(event, &mut model, &());

        // the app should ask the shell to render
        assert_effect!(cmd, Effect::Render(_));

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
        let app = App::default();

        // set up our initial model as though we've previously fetched the counter
        let mut model = Model {
            count: Count {
                value: 0,
                updated_at: Some(Utc.with_ymd_and_hms(2022, 12, 31, 23, 59, 0).unwrap()),
            },
        };

        // send a `Decrement` event to the app
        let mut update = app.update(Event::Decrement, &mut model, &());

        // the app should ask the shell to render the optimistic update
        assert_effect!(update, Effect::Render(_));

        // and send an HTTP post
        let mut request = update.effects().next().unwrap().expect_http();
        assert_eq!(
            &request.operation,
            &HttpRequest::post("https://crux-counter.fly.dev/dec").build()
        );

        // we are expecting our model to be updated "optimistically" before the
        // HTTP request completes, so the value should have been updated
        // but not the timestamp
        assert_eq!(
            model.count,
            Count {
                value: -1,
                updated_at: None
            }
        );

        // resolve the request with a simulated response from the web API
        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(r#"{ "value": -1, "updated_at": 1672531200000 }"#)
                    .build(),
            ))
            .unwrap();

        // this should generate a `Set` event
        let event = update.events().next().unwrap();
        assert!(matches!(event, Event::Set(_)));

        // send the `Set` event back to the app
        let mut update = app.update(event, &mut model, &());

        // this should generate an `Update` event
        let event = update.events().next().unwrap();
        assert_eq!(
            event,
            Event::Update(Count {
                value: -1,
                updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
            })
        );

        // send the `Update` event back to the app
        let mut update = app.update(event, &mut model, &());

        // the app should ask the shell to render
        assert_effect!(update, Effect::Render(_));

        // the model should be updated
        insta::assert_yaml_snapshot!(model, @r#"
        count:
          value: -1
          updated_at: "2023-01-01T00:00:00Z"
        "#);
    }

    #[test]
    fn server_sent_events() {
        let app = App::default();
        let mut model = Model::default();

        // start a SSE subscription to watch for updates from the server
        let mut cmd = app.update(Event::StartWatch, &mut model, &());

        // the app should request a Server-Sent Events stream
        let mut request = cmd.effects().next().unwrap().expect_server_sent_events();
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
        let event = cmd.events().next().unwrap();
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
                br#"data: {"value":2,"updated_at":1672531200000}

                    "#
                .to_vec(),
            ))
            .unwrap();

        // the app should emit another `Update` event with the new `Count`
        let event = cmd.events().next().unwrap();
        assert_eq!(
            event,
            Event::Update(Count {
                value: 2,
                updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
            })
        );
    }
}
