use chrono::{DateTime, NaiveDateTime, Utc};
use crux_core::render::Render;
use crux_http::Http;
use crux_macros::Effect;
use serde::{Deserialize, Serialize};
use url::Url;

const API_URL: &str = "https://crux-counter.fly.dev";

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
                caps.http
                    .get(API_URL)
                    .expect_json::<Counter>()
                    .send(Event::Set);
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
                caps.http.post(url.as_str()).expect_json().send(Event::Set);
            }
            Event::Decrement => {
                // optimistic update
                model.count.value -= 1;
                model.confirmed = Some(false);
                caps.render.render();

                // real update
                let base = Url::parse(API_URL).unwrap();
                let url = base.join("/dec").unwrap();
                caps.http.post(url.as_str()).expect_json().send(Event::Set);
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        model.into()
    }
}

#[derive(Default)]
pub struct Model {
    count: Counter,
    confirmed: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct ViewModel {
    pub text: String,
}

impl From<&Model> for ViewModel {
    fn from(model: &Model) -> Self {
        let updated_at = DateTime::<Utc>::from_utc(
            NaiveDateTime::from_timestamp_millis(model.count.updated_at).unwrap(),
            Utc,
        );
        let suffix = match model.confirmed {
            Some(true) => format!(" ({})", updated_at),
            Some(false) => " (pending)".to_string(),
            None => "".to_string(),
        };
        Self {
            text: model.count.value.to_string() + &suffix,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Event {
    Get,
    #[serde(skip)]
    Set(crux_http::Result<crux_http::Response<Counter>>),
    Increment,
    Decrement,
}

#[derive(Effect)]
pub struct Capabilities {
    pub http: Http<Event>,
    pub render: Render<Event>,
}

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Eq)]
pub struct Counter {
    value: isize,
    updated_at: i64,
}

#[cfg(test)]
mod tests {
    use super::{App, Event, Model};
    use crate::{Counter, Effect};
    use crux_core::{render::RenderOperation, testing::AppTester};
    use crux_http::{
        protocol::{HttpRequest, HttpResponse},
        testing::ResponseBuilder,
    };

    #[test]
    fn get_counter() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::Get, &mut model);

        let actual = &update.effects[0];
        let expected = &Effect::Http(HttpRequest {
            method: "GET".to_string(),
            url: "https://crux-counter.fly.dev/".to_string(),
        });
        assert_eq!(actual, expected);

        let update = update.effects[0].resolve(&HttpResponse {
            status: 200,
            body: serde_json::to_vec(&Counter {
                value: 1,
                updated_at: 1,
            })
            .unwrap(),
        });

        let actual = update.events;
        let expected = vec![Event::new_set(1, 1)];
        assert_eq!(actual, expected);
    }

    #[test]
    fn set_counter() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::new_set(1, 1), &mut model);

        let actual = &update.effects[0];
        let expected = &Effect::Render(RenderOperation);
        assert_eq!(actual, expected);

        let actual = model.count.value;
        let expected = 1;
        assert_eq!(actual, expected);

        let actual = model.confirmed;
        let expected = Some(true);
        assert_eq!(actual, expected);
    }

    #[test]
    fn increment_counter() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::Increment, &mut model);

        let actual = &update.effects[0];
        let expected = &Effect::Render(RenderOperation);
        assert_eq!(actual, expected);

        let actual = model.count.value;
        let expected = 1;
        assert_eq!(actual, expected);

        let actual = model.confirmed;
        let expected = Some(false);
        assert_eq!(actual, expected);

        let actual = &update.effects[1];
        let expected = &Effect::Http(HttpRequest {
            method: "POST".to_string(),
            url: "https://crux-counter.fly.dev/inc".to_string(),
        });
        assert_eq!(actual, expected);

        let update = update.effects[1].resolve(&HttpResponse {
            status: 200,
            body: serde_json::to_vec(&Counter {
                value: 1,
                updated_at: 1,
            })
            .unwrap(),
        });

        let actual = update.events;
        let expected = vec![Event::new_set(1, 1)];
        assert_eq!(actual, expected);
    }

    #[test]
    fn decrement_counter() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::Decrement, &mut model);

        let actual = &update.effects[0];
        let expected = &Effect::Render(RenderOperation);
        assert_eq!(actual, expected);

        let actual = model.count.value;
        let expected = -1;
        assert_eq!(actual, expected);

        let actual = model.confirmed;
        let expected = Some(false);
        assert_eq!(actual, expected);

        let actual = &update.effects[1];
        let expected = &Effect::Http(HttpRequest {
            method: "POST".to_string(),
            url: "https://crux-counter.fly.dev/dec".to_string(),
        });
        assert_eq!(actual, expected);

        let update = update.effects[1].resolve(&HttpResponse {
            status: 200,
            body: serde_json::to_vec(&Counter {
                value: -1,
                updated_at: 1,
            })
            .unwrap(),
        });

        let actual = update.events;
        let expected = vec![Event::new_set(-1, 1)];
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
