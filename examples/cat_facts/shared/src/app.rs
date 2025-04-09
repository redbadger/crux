pub mod platform;

use std::time::SystemTime;

use chrono::{DateTime, Utc};
use crux_http::{command::Http, protocol::HttpRequest};
use serde::{Deserialize, Serialize};

pub use crux_core::App;
use crux_core::{
    macros::effect,
    render::{self, RenderOperation},
    Command,
};
use crux_kv::{command::KeyValue, error::KeyValueError, KeyValueOperation};
use crux_platform::PlatformRequest;
use crux_time::{command::Time, TimeRequest};

const CAT_LOADING_URL: &str = "https://c.tenor.com/qACzaJ1EBVYAAAAd/tenor.gif";
const FACT_API_URL: &str = "https://catfact.ninja/fact";
const IMAGE_API_URL: &str = "https://crux-counter.fly.dev/cat";
const KEY: &str = "state";

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct CatFact {
    fact: String,
    length: i32,
}

impl CatFact {
    fn format(&self) -> String {
        format!("{} ({} bytes)", self.fact, self.length)
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct Model {
    cat_fact: Option<CatFact>,
    cat_image: Option<CatImage>,
    platform: platform::Model,
    time: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct CatImage {
    pub href: String,
}

impl Default for CatImage {
    fn default() -> Self {
        Self {
            href: CAT_LOADING_URL.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct ViewModel {
    pub fact: String,
    pub image: Option<CatImage>,
    pub platform: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Event {
    // events from the shell
    None,
    Clear,
    Get,
    Fetch,
    GetPlatform,
    Restore, // restore state

    // events local to the core
    #[serde(skip)]
    Platform(platform::Event),
    #[serde(skip)]
    SetState(Result<Option<Vec<u8>>, KeyValueError>), // receive the data to restore state with
    #[serde(skip)]
    CurrentTime(SystemTime),
    #[serde(skip)]
    SetFact(crux_http::Result<crux_http::Response<CatFact>>),
    #[serde(skip)]
    SetImage(crux_http::Result<crux_http::Response<CatImage>>),
}

#[derive(Default)]
pub struct CatFacts {
    platform: platform::App,
}

// ANCHOR: effect
#[effect(typegen)]
pub enum Effect {
    Http(HttpRequest),
    KeyValue(KeyValueOperation),
    Platform(PlatformRequest),
    Render(RenderOperation),
    Time(TimeRequest),
}

// ANCHOR_END: effect

impl App for CatFacts {
    type Model = Model;
    type Event = Event;
    type ViewModel = ViewModel;
    type Capabilities = ();
    type Effect = Effect;

    fn update(&self, msg: Event, model: &mut Model, _caps: &()) -> Command<Effect, Event> {
        self.update(msg, model)
    }

    fn view(&self, model: &Model) -> ViewModel {
        let fact = match (&model.cat_fact, &model.time) {
            (Some(fact), Some(time)) => format!("Fact from {}: {}", time, fact.format()),
            (Some(fact), _) => fact.format(),
            _ => "No fact".to_string(),
        };

        let platform =
            <platform::App as crux_core::App>::view(&self.platform, &model.platform).platform;

        ViewModel {
            platform,
            fact,
            image: model.cat_image.clone(),
        }
    }
}

impl CatFacts {
    fn update(&self, msg: Event, model: &mut Model) -> Command<Effect, Event> {
        match msg {
            Event::GetPlatform => self
                .platform
                .update(platform::Event::Get, &mut model.platform)
                .map_event(Event::Platform)
                .map_effect(Into::into),
            Event::Platform(msg) => self
                .platform
                .update(msg, &mut model.platform)
                .map_event(Event::Platform)
                .map_effect(Into::into),
            Event::Clear => {
                model.cat_fact = None;
                model.cat_image = None;
                let bytes = serde_json::to_vec(&model).unwrap();

                Command::all([
                    KeyValue::set(KEY, bytes).then_send(|_| Event::None),
                    render::render(),
                ])
            }
            Event::Get => {
                if let Some(_fact) = &model.cat_fact {
                    render::render()
                } else {
                    Command::event(Event::Fetch)
                }
            }
            Event::Fetch => {
                model.cat_image = Some(CatImage::default());

                // ANCHOR: command_all
                Command::all([
                    render::render(),
                    Http::get(FACT_API_URL)
                        .expect_json()
                        .build()
                        .then_send(Event::SetFact),
                    Http::get(IMAGE_API_URL)
                        .expect_json()
                        .build()
                        .then_send(Event::SetImage),
                ])
                // ANCHOR_END: command_all
            }
            Event::SetFact(Ok(mut response)) => {
                model.cat_fact = Some(response.take_body().unwrap());

                Time::now().then_send(Event::CurrentTime)
            }
            Event::SetImage(Ok(mut response)) => {
                model.cat_image = Some(response.take_body().unwrap());

                let bytes = serde_json::to_vec(&model).unwrap();

                Command::all([
                    render::render(),
                    KeyValue::set(KEY, bytes).then_send(|_| Event::None),
                ])
            }
            Event::SetFact(Err(_)) | Event::SetImage(Err(_)) => {
                // handle error
                Command::done()
            }
            Event::CurrentTime(time) => {
                let time: DateTime<Utc> = time.into();
                model.time = Some(time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));

                let bytes = serde_json::to_vec(&model).unwrap();

                Command::all([
                    render::render(),
                    KeyValue::set(KEY, bytes).then_send(|_| Event::None),
                ])
            }
            Event::Restore => KeyValue::get(KEY).then_send(Event::SetState),
            Event::SetState(Ok(Some(value))) => match serde_json::from_slice::<Model>(&value) {
                Ok(m) => {
                    *model = m;
                    render::render()
                }
                Err(_) => {
                    // handle error
                    Command::done()
                }
            },
            Event::SetState(Ok(None)) => {
                // no state to restore
                Command::done()
            }
            Event::SetState(Err(_)) => {
                // handle error
                Command::done()
            }
            Event::None => Command::done(),
        }
    }
}

impl From<platform::Effect> for Effect {
    fn from(effect: platform::Effect) -> Self {
        match effect {
            platform::Effect::Platform(request) => Effect::Platform(request),
            platform::Effect::Render(request) => Effect::Render(request),
        }
    }
}

#[cfg(test)]
mod tests {
    use crux_http::{
        protocol::{HttpRequest, HttpResponse, HttpResult},
        testing::ResponseBuilder,
    };
    use crux_kv::{value::Value, KeyValueOperation, KeyValueResponse, KeyValueResult};
    use crux_time::{Instant, TimeResponse};

    use super::*;

    #[test]
    fn fetch_results_in_set_fact_and_set_image() {
        let app = CatFacts::default();
        let mut model = Model::default();

        // send fetch event to app
        let mut fetch_command = app.update(Event::Fetch, &mut model);

        // receive render effect
        fetch_command.effects().next().unwrap().expect_render();

        // receive two HTTP effects, one to fetch the fact and one to fetch the image
        // we'll handle the fact request first
        let mut request = fetch_command.effects().next().unwrap().expect_http();
        assert_eq!(request.operation, HttpRequest::get(FACT_API_URL).build());

        let a_fact = CatFact {
            fact: "cats are good".to_string(),
            length: 13,
        };

        // resolve the request with a simulated response from the web API
        request
            .resolve(HttpResult::Ok(
                HttpResponse::ok()
                    .body(r#"{ "fact": "cats are good", "length": 13 }"#)
                    .build(),
            ))
            .expect("should resolve successfully");

        // check that the app emitted an (internal) event to update the model
        let event = fetch_command.events().next().unwrap();
        assert_eq!(
            event,
            Event::SetFact(Ok(ResponseBuilder::ok().body(a_fact.clone()).build()))
        );

        // Setting the fact should trigger a time effect
        let mut cmd = app.update(event, &mut model);

        let request = &mut cmd.effects().next().unwrap().expect_time();

        let instant = Instant::new(0, 0);
        request.resolve(TimeResponse::Now { instant }).unwrap();

        let event = cmd.events().next().unwrap();
        assert_eq!(event, Event::CurrentTime(instant.into()));

        // update the app with the current time event
        // and check that we get a render event ...
        let mut cmd = app.update(event, &mut model);
        cmd.effects().next().unwrap().expect_render();

        // ... and a key value set event
        let mut request = cmd.effects().next().unwrap().expect_key_value();
        assert_eq!(
            request.operation,
            KeyValueOperation::Set {
                key: KEY.to_string(),
                value: serde_json::to_vec(&model).unwrap()
            }
        );

        request
            .resolve(KeyValueResult::Ok {
                response: KeyValueResponse::Set {
                    previous: Value::None,
                },
            })
            .unwrap();

        // Now we'll handle the image
        let mut request = fetch_command.effects().next().unwrap().expect_http();
        assert_eq!(request.operation, HttpRequest::get(IMAGE_API_URL).build());

        let an_image = CatImage {
            href: "image_url".to_string(),
        };

        let response = HttpResult::Ok(HttpResponse::ok().body(r#"{"href":"image_url"}"#).build());
        request.resolve(response).unwrap();

        let event = fetch_command.events().next().unwrap();
        assert_eq!(
            event,
            Event::SetImage(Ok(ResponseBuilder::ok().body(an_image.clone()).build()))
        );

        let mut cmd = app.update(event, &mut model);
        cmd.effects().next().unwrap().expect_render();

        assert_eq!(model.cat_fact, Some(a_fact));
        assert_eq!(model.cat_image, Some(an_image));
    }
}
