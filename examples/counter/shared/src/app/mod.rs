use chrono::{DateTime, NaiveDateTime, Utc};
use crux_core::render::Render;
use crux_http::{Http, HttpError, HttpResponse};
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
                caps.http.get(Url::parse(API_URL).unwrap(), Event::Set);
            }
            Event::Set(Ok(HttpResponse { body, status: _ })) => {
                // TODO check status
                if let Some(body) = &body {
                    let Ok(counter) = serde_json::from_slice::<Counter>(body) else { return };
                    model.count = counter.value;
                    model.last_update = Some(Ok(counter.updated_at));
                }
                caps.render.render();
            }
            Event::Set(Err(error)) => {
                model.last_update = Some(Err(error));
                caps.render.render();
            }
            Event::Increment => {
                // optimistic update
                model.count += 1;
                model.last_update = Some(Ok(0));
                caps.render.render();

                // real update
                let base = Url::parse(API_URL).unwrap();
                let url = base.join("/inc");
                caps.http.post(url.unwrap(), Event::Set)
            }
            Event::Decrement => {
                // optimistic update
                model.count -= 1;
                model.last_update = Some(Ok(0));
                caps.render.render();

                // real update
                let base = Url::parse(API_URL).unwrap();
                let url = base.join("/dec");
                caps.http.post(url.unwrap(), Event::Set)
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        model.into()
    }
}

#[derive(Default)]
pub struct Model {
    count: i32,
    last_update: Option<Result<i64, HttpError>>,
}

#[derive(Serialize, Deserialize)]
pub struct ViewModel {
    pub text: String,
}

impl From<&Model> for ViewModel {
    fn from(model: &Model) -> Self {
        let suffix = match &model.last_update {
            Some(Ok(0)) => " (pending)".to_string(),
            Some(Ok(at)) => {
                let at = DateTime::<Utc>::from_utc(
                    NaiveDateTime::from_timestamp_millis(*at).unwrap(),
                    Utc,
                );
                format!(" ({at})")
            }
            Some(Err(error)) => format!(" (error: {error})"),
            None => "".to_string(),
        };
        Self {
            text: model.count.to_string() + &suffix,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Event {
    Get,
    Set(Result<HttpResponse, HttpError>),
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
    value: i32,
    updated_at: i64,
}

#[cfg(test)]
mod tests {
    use super::{App, Event, Model};
    use crate::{Counter, Effect};
    use crux_core::{render::RenderOperation, testing::AppTester};
    use crux_http::{HttpError, HttpRequest, HttpResponse};

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

        let body = Some(
            serde_json::to_vec(&Counter {
                value: 1,
                updated_at: 1,
            })
            .unwrap(),
        );
        let update = update.effects[0].resolve(&Ok::<_, HttpError>(HttpResponse {
            status: 200,
            body: body.clone(),
        }));

        let actual = update.events;
        let expected = vec![Event::Set(Ok(HttpResponse { status: 200, body }))];
        assert_eq!(actual, expected);
    }

    #[test]
    fn set_counter() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let body = Some(
            serde_json::to_vec(&Counter {
                value: 1,
                updated_at: 1,
            })
            .unwrap(),
        );

        let update = app.update(
            Event::Set(Ok(HttpResponse { status: 200, body })),
            &mut model,
        );

        let actual = &update.effects[0];
        let expected = &Effect::Render(RenderOperation);
        assert_eq!(actual, expected);

        let actual = model.count;
        let expected = 1;
        assert_eq!(actual, expected);

        let actual = model.last_update;
        let expected = Some(Ok(1));
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

        let actual = model.count;
        let expected = 1;
        assert_eq!(actual, expected);

        let actual = model.last_update;
        let expected = Some(Ok(0));
        assert_eq!(actual, expected);

        let actual = &update.effects[1];
        let expected = &Effect::Http(HttpRequest {
            method: "POST".to_string(),
            url: "https://crux-counter.fly.dev/inc".to_string(),
        });
        assert_eq!(actual, expected);

        let update = update.effects[1].resolve(&Ok::<_, HttpError>(HttpResponse {
            status: 200,
            body: Some(
                serde_json::to_vec(&Counter {
                    value: 1,
                    updated_at: 1,
                })
                .unwrap(),
            ),
        }));

        let body = Some(
            serde_json::to_vec(&Counter {
                value: 1,
                updated_at: 1,
            })
            .unwrap(),
        );

        let actual = update.events;
        let expected = vec![Event::Set(Ok(HttpResponse { status: 200, body }))];
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

        let actual = model.count;
        let expected = -1;
        assert_eq!(actual, expected);

        let actual = model.last_update;
        let expected = Some(Ok(0));
        assert_eq!(actual, expected);

        let actual = &update.effects[1];
        let expected = &Effect::Http(HttpRequest {
            method: "POST".to_string(),
            url: "https://crux-counter.fly.dev/dec".to_string(),
        });
        assert_eq!(actual, expected);

        let update = update.effects[1].resolve(&Ok::<_, HttpError>(HttpResponse {
            status: 200,
            body: Some(
                serde_json::to_vec(&Counter {
                    value: -1,
                    updated_at: 1,
                })
                .unwrap(),
            ),
        }));

        let body = Some(
            serde_json::to_vec(&Counter {
                value: -1,
                updated_at: 1,
            })
            .unwrap(),
        );

        let actual = update.events;
        let expected = vec![Event::Set(Ok(HttpResponse { status: 200, body }))];
        assert_eq!(actual, expected);
    }
}
