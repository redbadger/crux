use chrono::{DateTime, Utc, serde::ts_milliseconds_option::deserialize as ts_milliseconds_option};
use crux_core::{
    Command,
    capability::Operation,
    macros::effect,
    render::{RenderOperation, render},
};
use crux_http::{HttpError, command::Http, protocol::HttpRequest};
use facet::Facet;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    capabilities::RandomNumberRequest,
    sse::{self, SseRequest},
};

const API_URL: &str = "https://crux-counter.fly.dev";

#[derive(Facet, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum HttpResult<T, E> {
    Ok(T),
    Err(E),
}

impl<T> From<crux_http::Result<crux_http::Response<T>>>
    for HttpResult<crux_http::Response<T>, HttpError>
{
    fn from(value: crux_http::Result<crux_http::Response<T>>) -> Self {
        match value {
            Ok(response) => HttpResult::Ok(response),
            Err(error) => HttpResult::Err(error),
        }
    }
}

#[derive(Default, Serialize)]
pub struct Model {
    count: Count,
}

#[derive(Facet, Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Count {
    value: isize,
    #[serde(deserialize_with = "ts_milliseconds_option")]
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Facet, Serialize, Deserialize, Debug, Clone, Default)]
#[facet(namespace = "view_model")]
pub struct ViewModel {
    pub text: String,
    pub confirmed: bool,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Event {
    // events from the shell
    Test,
    Get,
    Increment,
    Decrement,
    Random,
    StartWatch,

    // events local to the core
    #[serde(skip)]
    #[facet(skip)]
    TestSet(TestResponse),
    #[serde(skip)]
    #[facet(skip)]
    Set(HttpResult<crux_http::Response<Count>, HttpError>),
    #[serde(skip)]
    #[facet(skip)]
    Update(Count),
    #[serde(skip)]
    UpdateBy(isize),
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TestRequest;

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TestResponse(isize);

impl Operation for TestRequest {
    type Output = TestResponse;
}

#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
    ServerSentEvents(SseRequest),
    Random(RandomNumberRequest),
    Test(TestRequest),
}

#[derive(Default)]
pub struct App;

impl crux_core::App for App {
    type Model = Model;
    type Event = Event;
    type ViewModel = ViewModel;
    type Capabilities = ();
    type Effect = Effect;

    #[allow(clippy::too_many_lines)]
    fn update(
        &self,
        msg: Self::Event,
        model: &mut Self::Model,
        _caps: &Self::Capabilities,
    ) -> Command<Effect, Event> {
        match msg {
            Event::Test => Command::request_from_shell(TestRequest).then_send(Event::TestSet),
            Event::TestSet(TestResponse(value)) => {
                model.count = Count {
                    value,
                    updated_at: None,
                };
                Command::done()
            }
            Event::Get => Http::get(API_URL)
                .expect_json()
                .build()
                .map(Into::into)
                .then_send(Event::Set),
            Event::Set(HttpResult::Ok(mut response)) => {
                let count = response.take_body().unwrap();
                Command::event(Event::Update(count))
            }
            Event::Set(HttpResult::Err(e)) => {
                panic!("Oh no something went wrong: {e:?}");
            }
            Event::Update(count) => {
                model.count = count;
                render()
            }
            Event::UpdateBy(change) => {
                if change == 0 {
                    return render();
                }

                model.count = Count {
                    value: model.count.value + change,
                    updated_at: None,
                };

                let call_api = {
                    // Don't look at this too closely.
                    // The API doesn't support adjusting by a delta, so we just call it
                    // several times. That's why the range is hardcoded. Don't @ me
                    let base = Url::parse(API_URL).unwrap();

                    let url = if change < 0 {
                        base.join("/dec").unwrap()
                    } else {
                        base.join("/inc").unwrap()
                    };

                    let n = change.unsigned_abs();

                    Command::new(|ctx| async move {
                        let futures = (0..n).map(|_| {
                            Http::post(url.clone())
                                .expect_json::<Count>()
                                .build()
                                .into_future(ctx.clone())
                        });

                        let result: Result<Vec<crux_http::Response<Count>>, crux_http::HttpError> =
                            join_all(futures).await.into_iter().collect();

                        let latest = result.map(|counts| {
                            counts
                                .into_iter()
                                .max_by_key(|c| c.body().unwrap().updated_at.unwrap())
                                .unwrap()
                        });

                        ctx.send_event(Event::Set(latest.into()));
                    })
                };

                render().and(call_api)
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
                    Http::post(url)
                        .expect_json()
                        .build()
                        .map(Into::into)
                        .then_send(Event::Set)
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
                    Http::post(url)
                        .expect_json()
                        .build()
                        .map(Into::into)
                        .then_send(Event::Set)
                };

                render().and(call_api)
            }
            Event::Random => Command::request_from_shell(RandomNumberRequest(-5, 5))
                .map(|out| out.0)
                .then_send(Event::UpdateBy),
            Event::StartWatch => {
                let base = Url::parse(API_URL).unwrap();
                let url = base.join("/sse").unwrap();
                sse::get(url).then_send(Event::Update)
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

    use super::{App, Count, Effect, Event, Model};
    use crate::{
        RandomNumber, RandomNumberRequest,
        sse::{SseRequest, SseResponse},
    };

    use crux_core::{App as _, assert_effect};
    use crux_http::{
        protocol::{HttpRequest, HttpResponse, HttpResult},
        testing::ResponseBuilder,
    };

    // ANCHOR: simple_tests
    /// Test that a `Get` event causes the app to fetch the current
    /// counter value from the web API
    #[test]
    fn get_counter() {
        let app = App;
        let mut model = Model::default();

        // send a `Get` event to the app
        let mut cmd = app.update(Event::Get, &mut model, &());

        // the app should emit an HTTP request to fetch the counter
        let (operation, mut request) = cmd.effects().next().unwrap().expect_http().split();

        // and the request should be a GET to the correct URL
        assert_eq!(
            &operation,
            &HttpRequest::get("https://crux-counter.fly.dev/").build()
        );

        // resolve the request with a simulated response from the web API
        let response = HttpResponse::ok()
            .body(r#"{ "value": 1, "updated_at": 1672531200000 }"#)
            .build();
        request
            .resolve(crux_http::protocol::HttpResult::Ok(response))
            .unwrap();

        // the app should emit a `Set` event with the HTTP response
        let actual = cmd.events().next().unwrap();
        let response = ResponseBuilder::ok()
            .body(Count {
                value: 1,
                updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
            })
            .build();
        let expected = Event::Set(super::HttpResult::Ok(response));
        assert_eq!(actual, expected);

        // send the `Set` event back to the app
        let mut cmd = app.update(actual, &mut model, &());

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
        assert!(view.confirmed);
    }
    // ANCHOR_END: simple_tests

    // Test that an `Increment` event causes the app to increment the counter
    #[test]
    fn increment_counter() {
        let app = App;

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
        let response = HttpResponse::ok()
            .body(r#"{ "value": 2, "updated_at": 1672531200000 }"#)
            .build();
        request
            .resolve(crux_http::protocol::HttpResult::Ok(response))
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
        let app = App;

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
        let response = HttpResponse::ok()
            .body(r#"{ "value": -1, "updated_at": 1672531200000 }"#)
            .build();
        request
            .resolve(crux_http::protocol::HttpResult::Ok(response))
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
        let app = App;
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

    #[test]
    fn random_change() {
        let app = App;
        let mut model = Model::default();

        let mut cmd = app.update(Event::Random, &mut model, &());

        // the app should request a random number from the web API
        let mut request = cmd.effects().next().unwrap().expect_random();

        assert_eq!(request.operation, RandomNumberRequest(-5, 5));
        request.resolve(RandomNumber(-2)).unwrap();

        // And start an UpdateBy the number

        let event = cmd.events().next().unwrap();
        assert_eq!(event, Event::UpdateBy(-2));

        let mut cmd = app.update(event, &mut model, &());

        // we should now see two decrement http requests

        cmd.effects().next().unwrap().expect_render();
        let mut request_one = cmd.effects().next().unwrap().expect_http();
        let mut request_two = cmd.effects().next().unwrap().expect_http();
        assert_eq!(
            &request_one.operation,
            &HttpRequest::post("https://crux-counter.fly.dev/dec").build()
        );
        assert_eq!(
            &request_two.operation,
            &HttpRequest::post("https://crux-counter.fly.dev/dec").build()
        );

        request_one
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(r#"{ "value": 7, "updated_at": 1672531200000 }"#)
                    .build(),
            ))
            .expect("should resolve");
        request_two
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(r#"{ "value": 9, "updated_at": 1672531200001 }"#) // this is the latest
                    .build(),
            ))
            .expect("should resolve");

        // And once we process the event train

        let event = cmd.events().next().unwrap();
        let mut cmd = app.update(event, &mut model, &());

        let event = cmd.events().next().unwrap();
        let mut cmd = app.update(event, &mut model, &());

        cmd.effects().next().unwrap().expect_render();
        assert!(cmd.is_done());

        // The latest count wins

        assert_eq!(model.count.value, 9);
    }

    #[test]
    fn random_change_with_0() {
        let app = App;
        let mut model = Model::default();

        model.count.value = 3;
        model.count.updated_at = Some(Utc::now());

        let mut cmd = app.update(Event::Random, &mut model, &());

        // the app should request a random number from the web API
        let mut request = cmd.effects().next().unwrap().expect_random();

        assert_eq!(request.operation, RandomNumberRequest(-5, 5));
        request.resolve(RandomNumber(0)).unwrap();

        // And start an UpdateBy the number

        let event = cmd.events().next().unwrap();
        assert_eq!(event, Event::UpdateBy(0));

        let mut cmd = app.update(event, &mut model, &());
        cmd.effects().next().unwrap().expect_render();
        assert!(cmd.is_done());

        // The latest count wins

        assert_eq!(model.count.value, 3);
        assert!(model.count.updated_at.is_some());
    }
}
