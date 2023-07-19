use crate::capabilities::sse::ServerSentEvents;
use chrono::{DateTime, NaiveDateTime, Utc};
use crux_core::render::Render;
use crux_http::Http;
use crux_macros::Effect;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

const API_URL: &str = "https://crux-counter.fly.dev";

#[derive(Default)]
pub struct Model {
    count: Counter,
    confirmed: Option<bool>,
}

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
    SendUuid(Uuid),

    // events local to the core
    #[serde(skip)]
    Set(crux_http::Result<crux_http::Response<Counter>>),
    WatchUpdate(Counter),
}

#[cfg_attr(feature = "typegen", derive(crux_macros::Export))]
#[derive(Effect)]
pub struct Capabilities {
    pub http: Http<Event>,
    pub render: Render<Event>,
    pub sse: ServerSentEvents<Event>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Counter {
    value: isize,
    updated_at: i64,
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
            Event::Set(Ok(mut counter)) => {
                model.count = counter.take_body().unwrap();
                model.confirmed = Some(true);
                caps.render.render();
            }
            Event::Set(Err(_)) => {
                panic!("Oh no something went wrong");
            }
            Event::Increment => {
                // optimistic update
                model.count.value += 1;
                model.confirmed = Some(false);
                caps.render.render();

                // real update
                let base = Url::parse(API_URL).unwrap();
                let url = base.join("/inc").unwrap();
                caps.http.post(url).expect_json().send(Event::Set);
            }
            Event::Decrement => {
                // optimistic update
                model.count.value -= 1;
                model.confirmed = Some(false);
                caps.render.render();

                // real update
                let base = Url::parse(API_URL).unwrap();
                let url = base.join("/dec").unwrap();
                caps.http.post(url).expect_json().send(Event::Set);
            }
            Event::StartWatch => {
                let base = Url::parse(API_URL).unwrap();
                let url = base.join("/sse").unwrap();
                caps.sse.get_json(url, Event::WatchUpdate);
            }
            Event::WatchUpdate(count) => {
                model.count = count;
                model.confirmed = Some(true);
                caps.render.render();
            }
            Event::SendUuid(_uuid) => {
                // No-op
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        let updated_at = DateTime::<Utc>::from_utc(
            NaiveDateTime::from_timestamp_millis(model.count.updated_at).unwrap(),
            Utc,
        ); // .format("updated at %H:%M:%S");

        let suffix = match model.confirmed {
            Some(true) => format!(" ({updated_at})"),
            Some(false) => " (pending)".to_string(),
            None => "".to_string(),
        };

        Self::ViewModel {
            text: model.count.value.to_string() + &suffix,
            confirmed: model.confirmed.unwrap_or(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{App, Event, Model};
    use crate::capabilities::sse::SseRequest;
    use crate::{Counter, Effect};
    use assert_let_bind::assert_let;
    use crux_core::{assert_effect, testing::AppTester};
    use crux_http::{
        protocol::{HttpRequest, HttpResponse},
        testing::ResponseBuilder,
    };

    // ANCHOR: simple_tests
    #[test]
    fn get_counter() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let mut update = app.update(Event::Get, &mut model);

        assert_let!(Effect::Http(request), update.effects_mut().next().unwrap());

        let actual = &request.operation;
        let expected = &HttpRequest::get("https://crux-counter.fly.dev/").build();
        assert_eq!(actual, expected);

        let response = HttpResponse::ok()
            .json(&Counter {
                value: 1,
                updated_at: 1,
            })
            .build();

        let update = app.resolve(request, response).expect("an update");

        let actual = update.events;
        let expected = vec![Event::new_set(1, 1)];
        assert_eq!(actual, expected);
    }

    #[test]
    fn set_counter() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::new_set(1, 1), &mut model);

        assert_effect!(update, Effect::Render(_));

        let actual = model.count.value;
        let expected = 1;
        assert_eq!(actual, expected);

        let actual = model.confirmed;
        let expected = Some(true);
        assert_eq!(actual, expected);
    }
    // ANCHOR_END: simple_tests

    #[test]
    fn increment_counter() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let mut update = app.update(Event::Increment, &mut model);

        assert_effect!(update, Effect::Render(_));

        let actual = model.count.value;
        let expected = 1;
        assert_eq!(actual, expected);

        let actual = model.confirmed;
        let expected = Some(false);
        assert_eq!(actual, expected);

        assert_let!(Effect::Http(request), update.effects_mut().nth(1).unwrap());
        let actual = &request.operation;
        let expected = &HttpRequest::post("https://crux-counter.fly.dev/inc").build();

        assert_eq!(actual, expected);

        let response = HttpResponse::ok()
            .json(&Counter {
                value: 1,
                updated_at: 1,
            })
            .build();

        let update = app.resolve(request, response).expect("Update to succeed");

        let actual = update.events;
        let expected = vec![Event::new_set(1, 1)];
        assert_eq!(actual, expected);
    }

    #[test]
    fn decrement_counter() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let mut update = app.update(Event::Decrement, &mut model);

        assert_effect!(update, Effect::Render(_));

        let actual = model.count.value;
        let expected = -1;
        assert_eq!(actual, expected);

        let actual = model.confirmed;
        let expected = Some(false);
        assert_eq!(actual, expected);

        assert_let!(Effect::Http(request), update.effects_mut().nth(1).unwrap());
        let actual = &request.operation;
        let expected = &HttpRequest::post("https://crux-counter.fly.dev/dec").build();
        assert_eq!(actual, expected);

        let response = HttpResponse::ok()
            .json(&Counter {
                value: -1,
                updated_at: 1,
            })
            .build();

        let update = app.resolve(request, response).expect("a successful update");

        let actual = update.events;
        let expected = vec![Event::new_set(-1, 1)];
        assert_eq!(actual, expected);
    }

    #[test]
    fn get_sse() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::StartWatch, &mut model);

        assert_let!(
            Effect::ServerSentEvents(request),
            update.effects().next().unwrap()
        );
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

        let count = Counter {
            value: 1,
            updated_at: 1,
        };
        let event = Event::WatchUpdate(count);

        let update = app.update(event, &mut model);

        assert_let!(Effect::Render(_), update.effects().next().unwrap());

        let actual = model.count.value;
        let expected = 1;
        assert_eq!(actual, expected);

        let actual = model.confirmed;
        let expected = Some(true);
        assert_eq!(actual, expected);
    }

    impl Event {
        fn new_set(value: isize, updated_at: i64) -> Event {
            let response = ResponseBuilder::ok()
                .body(Counter { value, updated_at })
                .build();

            Event::Set(Ok(response))
        }
    }
}
