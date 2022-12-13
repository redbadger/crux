use crux_core::render::Render;
use crux_http::Http;
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
                caps.http.get_json(Url::parse(API_URL).unwrap(), Event::Set);
            }
            Event::Set(counter) => {
                model.count = counter.value;
                caps.render.render();
            }
            Event::Increment => {
                // optimistic update
                model.count += 1;
                caps.render.render();

                // real update
                let base = Url::parse(API_URL).unwrap();
                let url = base.join("/inc");
                caps.http.post(url.unwrap(), Event::Set)
            }
            Event::Decrement => {
                // optimistic update
                model.count -= 1;
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
    count: isize,
}

#[derive(Serialize, Deserialize)]
pub struct ViewModel {
    pub count: isize,
}

impl From<&Model> for ViewModel {
    fn from(model: &Model) -> Self {
        Self { count: model.count }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Event {
    Get,
    Set(Counter),
    Increment,
    Decrement,
}

pub struct Capabilities {
    pub http: Http<Event>,
    pub render: Render<Event>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Counter {
    value: isize,
}

#[cfg(test)]
mod tests {
    use super::{App, Event, Model};
    use crate::{Counter, Effect};
    use crux_core::testing::AppTester;
    use crux_http::{HttpRequest, HttpResponse};

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
            body: serde_json::to_vec(&Counter { value: 1 }).unwrap(),
        });

        let actual = update.events;
        let expected = vec![Event::Set(Counter { value: 1 })];
        assert_eq!(actual, expected);
    }

    #[test]
    fn set_counter() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::Set(Counter { value: 1 }), &mut model);

        let actual = &update.effects[0];
        let expected = &Effect::Render;
        assert_eq!(actual, expected);

        let actual = model.count;
        let expected = 1;
        assert_eq!(actual, expected);
    }

    #[test]
    fn increment_counter() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::Increment, &mut model);

        let actual = &update.effects[0];
        let expected = &Effect::Render;
        assert_eq!(actual, expected);

        let actual = model.count;
        let expected = 1;
        assert_eq!(actual, expected);

        let actual = &update.effects[1];
        let expected = &Effect::Http(HttpRequest {
            method: "POST".to_string(),
            url: "https://crux-counter.fly.dev/inc".to_string(),
        });
        assert_eq!(actual, expected);

        let update = update.effects[1].resolve(&HttpResponse {
            status: 200,
            body: serde_json::to_vec(&Counter { value: 1 }).unwrap(),
        });

        let actual = update.events;
        let expected = vec![Event::Set(Counter { value: 1 })];
        assert_eq!(actual, expected);
    }

    #[test]
    fn decrement_counter() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::Decrement, &mut model);

        let actual = &update.effects[0];
        let expected = &Effect::Render;
        assert_eq!(actual, expected);

        let actual = model.count;
        let expected = -1;
        assert_eq!(actual, expected);

        let actual = &update.effects[1];
        let expected = &Effect::Http(HttpRequest {
            method: "POST".to_string(),
            url: "https://crux-counter.fly.dev/dec".to_string(),
        });
        assert_eq!(actual, expected);

        let update = update.effects[1].resolve(&HttpResponse {
            status: 200,
            body: serde_json::to_vec(&Counter { value: 1 }).unwrap(),
        });

        let actual = update.events;
        let expected = vec![Event::Set(Counter { value: 1 })];
        assert_eq!(actual, expected);
    }
}
