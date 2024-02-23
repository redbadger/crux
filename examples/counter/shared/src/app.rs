use crate::capabilities::sse::ServerSentEvents;
use chrono::{serde::ts_milliseconds_option::deserialize as ts_milliseconds_option, DateTime, Utc};
use crux_core::render::Render;
use crux_http::Http;
use serde::{Deserialize, Serialize};
use url::Url;

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

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[cfg_attr(feature = "typegen", derive(crux_core::macros::Export))]
#[derive(crux_core::macros::Effect)]
pub struct Capabilities {
    pub render: Render<Event>,
    pub http: Http<Event>,
    pub sse: ServerSentEvents<Event>,
}

#[derive(Default)]
pub struct App;

impl crux_core::App for App {
    type Model = Model;
    type Event = Event;
    type ViewModel = ViewModel;
    type Capabilities = Capabilities;

    fn update(&self, msg: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) {
        match msg {
            Event::Get => {
                caps.http.get(API_URL).expect_json().send(Event::Set);
            }
            Event::Set(Ok(mut response)) => {
                let count = response.take_body().unwrap();
                self.update(Event::Update(count), model, caps);
            }
            Event::Set(Err(e)) => {
                panic!("Oh no something went wrong: {e:?}");
            }
            Event::Update(count) => {
                model.count = count;
                caps.render.render();
            }
            Event::Increment => {
                // optimistic update
                model.count = Count {
                    value: model.count.value + 1,
                    updated_at: None,
                };
                caps.render.render();

                // real update
                let base = Url::parse(API_URL).unwrap();
                let url = base.join("/inc").unwrap();
                caps.http.post(url).expect_json().send(Event::Set);
            }
            Event::Decrement => {
                // optimistic update
                model.count = Count {
                    value: model.count.value - 1,
                    updated_at: None,
                };
                caps.render.render();

                // real update
                let base = Url::parse(API_URL).unwrap();
                let url = base.join("/dec").unwrap();
                caps.http.post(url).expect_json().send(Event::Set);
            }
            Event::StartWatch => {
                let base = Url::parse(API_URL).unwrap();
                let url = base.join("/sse").unwrap();
                caps.sse.get_json(url, Event::Update);
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
    use super::{App, Event, Model};
    use crate::capabilities::sse::SseRequest;
    use crate::{Count, Effect};
    use assert_let_bind::assert_let;
    use chrono::{TimeZone, Utc};
    use crux_core::{assert_effect, testing::AppTester};
    use crux_http::protocol::HttpResult;
    use crux_http::{
        protocol::{HttpRequest, HttpResponse},
        testing::ResponseBuilder,
    };

    // ANCHOR: simple_tests
    /// Test that a `Get` event causes the app to fetch the current
    /// counter value from the web API
    #[test]
    fn get_counter() {
        // instantiate our app via the test harness, which gives us access to the model
        let app = AppTester::<App, _>::default();

        // set up our initial model
        let mut model = Model::default();

        // send a `Get` event to the app
        let mut update = app.update(Event::Get, &mut model);

        // check that the app emitted an HTTP request,
        // capturing the request in the process
        assert_let!(Effect::Http(request), &mut update.effects[0]);

        // check that the request is a GET to the correct URL
        let actual = &request.operation;
        let expected = &HttpRequest::get("https://crux-counter.fly.dev/").build();
        assert_eq!(actual, expected);

        // resolve the request with a simulated response from the web API
        let response = HttpResponse::ok()
            .body(r#"{ "value": 1, "updated_at": 1672531200000 }"#)
            .build();
        let update = app
            .resolve(request, HttpResult::Ok(response))
            .expect("an update");

        // check that the app emitted an (internal) event to update the model
        let actual = update.events;
        let expected = vec![Event::Set(Ok(ResponseBuilder::ok()
            .body(Count {
                value: 1,
                updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
            })
            .build()))];
        assert_eq!(actual, expected);
    }

    /// Test that a `Set` event causes the app to update the model
    #[test]
    fn set_counter() {
        // instantiate our app via the test harness, which gives us access to the model
        let app = AppTester::<App, _>::default();

        // set up our initial model
        let mut model = Model::default();

        // send a `Set` event (containing the HTTP response) to the app
        let update = app.update(
            Event::Set(Ok(ResponseBuilder::ok()
                .body(Count {
                    value: 1,
                    updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
                })
                .build())),
            &mut model,
        );

        // check that the app asked the shell to render
        assert_effect!(update, Effect::Render(_));

        // check that the view has been updated correctly
        insta::assert_yaml_snapshot!(app.view(&model), @r###"
        ---
        text: "1 (2023-01-01 00:00:00 UTC)"
        confirmed: true
        "###);
    }
    // ANCHOR_END: simple_tests

    // Test that an `Increment` event causes the app to increment the counter
    #[test]
    fn increment_counter() {
        // instantiate our app via the test harness, which gives us access to the model
        let app = AppTester::<App, _>::default();

        // set up our initial model as though we've previously fetched the counter
        let mut model = Model {
            count: Count {
                value: 1,
                updated_at: Some(Utc.with_ymd_and_hms(2022, 12, 31, 23, 59, 0).unwrap()),
            },
        };

        // send an `Increment` event to the app
        let mut update = app.update(Event::Increment, &mut model);

        // check that the app asked the shell to render
        assert_effect!(update, Effect::Render(_));

        // we are expecting our model to be updated "optimistically" before the
        // HTTP request completes, so the value should have been updated
        // but not the timestamp
        insta::assert_yaml_snapshot!(model, @r###"
        ---
        count:
          value: 2
          updated_at: ~
        "###);

        // check that the app also emitted an HTTP request,
        // capturing the request in the process
        assert_let!(Effect::Http(request), &mut update.effects[1]);

        // check that the request is a POST to the correct URL
        let actual = &request.operation;
        let expected = &HttpRequest::post("https://crux-counter.fly.dev/inc").build();
        assert_eq!(actual, expected);

        // resolve the request with a simulated response from the web API
        let response = HttpResponse::ok()
            .body(r#"{ "value": 2, "updated_at": 1672531200000 }"#)
            .build();
        let update = app
            .resolve(request, HttpResult::Ok(response))
            .expect("Update to succeed");

        // send the generated (internal) `Set` event back into the app
        app.update(update.events[0].clone(), &mut model);

        // check that the model has been updated correctly
        insta::assert_yaml_snapshot!(model, @r###"
        ---
        count:
          value: 2
          updated_at: "2023-01-01T00:00:00Z"
        "###);
    }

    /// Test that a `Decrement` event causes the app to decrement the counter
    #[test]
    fn decrement_counter() {
        // instantiate our app via the test harness, which gives us access to the model
        let app = AppTester::<App, _>::default();

        // set up our initial model as though we've previously fetched the counter
        let mut model = Model {
            count: Count {
                value: 0,
                updated_at: Some(Utc.with_ymd_and_hms(2022, 12, 31, 23, 59, 0).unwrap()),
            },
        };

        // send a `Decrement` event to the app
        let mut update = app.update(Event::Decrement, &mut model);

        // check that the app asked the shell to render
        assert_effect!(update, Effect::Render(_));

        // we are expecting our model to be updated "optimistically" before the
        // HTTP request completes, so the value should have been updated
        // but not the timestamp
        insta::assert_yaml_snapshot!(model, @r###"
        ---
        count:
          value: -1
          updated_at: ~
        "###);

        // check that the app also emitted an HTTP request,
        // capturing the request in the process
        assert_let!(Effect::Http(request), &mut update.effects[1]);

        // check that the request is a POST to the correct URL
        let actual = &request.operation;
        let expected = &HttpRequest::post("https://crux-counter.fly.dev/dec").build();
        assert_eq!(actual, expected);

        // resolve the request with a simulated response from the web API
        let response = HttpResponse::ok()
            .body(r#"{ "value": -1, "updated_at": 1672531200000 }"#)
            .build();
        let update = app
            .resolve(request, HttpResult::Ok(response))
            .expect("a successful update");

        // run the event loop in order to send the (internal) `Set` event
        // back into the app
        for event in update.events {
            app.update(event, &mut model);
        }

        // check that the model has been updated correctly
        insta::assert_yaml_snapshot!(model, @r###"
        ---
        count:
          value: -1
          updated_at: "2023-01-01T00:00:00Z"
        "###);
    }

    #[test]
    fn get_sse() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::StartWatch, &mut model);

        assert_let!(Effect::ServerSentEvents(request), &update.effects[0]);

        let actual = &request.operation;
        let expected = &SseRequest {
            url: "https://crux-counter.fly.dev/sse".to_string(),
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn set_sse() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let count = Count {
            value: 1,
            updated_at: Some(Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap()),
        };
        let event = Event::Update(count);

        let update = app.update(event, &mut model);

        assert_effect!(update, Effect::Render(_));

        // check that the model has been updated correctly
        insta::assert_yaml_snapshot!(model, @r###"
        ---
        count:
          value: 1
          updated_at: "2023-01-01T00:00:00Z"
        "###);
    }
}
