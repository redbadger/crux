use crux_core::{render::Render, App};
use crux_http::Http;
use serde::{Deserialize, Serialize};
use url::Url;

const API_URL: &str = "https://https://crux-counter.fly.dev";

#[derive(Default)]
pub struct MyApp;

impl App for MyApp {
    type Model = MyModel;

    type Event = MyEvent;

    type ViewModel = MyViewModel;

    type Capabilities = MyCapabilities;

    fn update(&self, msg: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) {
        match msg {
            MyEvent::Get => {
                caps.http
                    .get_json(Url::parse(API_URL).unwrap(), MyEvent::Set);
            }
            MyEvent::Set(counter) => {
                model.count = counter.value;
                caps.render.render();
            }
            MyEvent::Increment => {
                let base = Url::parse(API_URL).unwrap();
                let url = base.join("/inc");
                caps.http.post(url.unwrap(), MyEvent::Set)
            }
            MyEvent::Decrement => {
                let base = Url::parse(API_URL).unwrap();
                let url = base.join("/dec");
                caps.http.post(url.unwrap(), MyEvent::Set)
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        model.into()
    }
}

#[derive(Default)]
pub struct MyModel {
    count: isize,
}

#[derive(Serialize)]
pub struct MyViewModel {
    count: isize,
}

impl From<&MyModel> for MyViewModel {
    fn from(model: &MyModel) -> Self {
        Self { count: model.count }
    }
}

#[derive(Deserialize)]
pub enum MyEvent {
    Get,
    Set(Counter),
    Increment,
    Decrement,
}

pub struct MyCapabilities {
    pub http: Http<MyEvent>,
    pub render: Render<MyEvent>,
}

#[derive(Deserialize)]
pub struct Counter {
    value: isize,
}
