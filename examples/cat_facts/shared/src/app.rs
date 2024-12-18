pub mod platform;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use crux_core::App;
use crux_core::{render::Render, Capability};
use crux_http::Http;
use crux_kv::{error::KeyValueError, KeyValue};
use crux_platform::Platform;
use crux_time::{Time, TimeResponse};

use platform::Capabilities;

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
    CurrentTime(TimeResponse),
    #[serde(skip)]
    SetFact(crux_http::Result<crux_http::Response<CatFact>>),
    #[serde(skip)]
    SetImage(crux_http::Result<crux_http::Response<CatImage>>),
}

#[derive(Default)]
pub struct CatFacts {
    platform: platform::App,
}

#[cfg_attr(feature = "typegen", derive(crux_core::macros::Export))]
#[derive(crux_core::macros::Effect)]
pub struct CatFactCapabilities {
    http: Http<Event>,
    key_value: KeyValue<Event>,
    platform: Platform<Event>,
    render: Render<Event>,
    time: Time<Event>,
}

// Allow easily using Platform as a submodule
impl From<&CatFactCapabilities> for Capabilities {
    fn from(incoming: &CatFactCapabilities) -> Self {
        Capabilities {
            platform: incoming.platform.map_event(super::Event::Platform),
            render: incoming.render.map_event(super::Event::Platform),
        }
    }
}

impl App for CatFacts {
    type Model = Model;
    type Event = Event;
    type ViewModel = ViewModel;
    type Capabilities = CatFactCapabilities;

    fn update(&self, msg: Event, model: &mut Model, caps: &CatFactCapabilities) {
        match msg {
            Event::GetPlatform => {
                self.platform
                    .update(platform::Event::Get, &mut model.platform, &caps.into())
            }
            Event::Platform(msg) => self.platform.update(msg, &mut model.platform, &caps.into()),
            Event::Clear => {
                model.cat_fact = None;
                model.cat_image = None;
                let bytes = serde_json::to_vec(&model).unwrap();

                caps.key_value.set(KEY.to_string(), bytes, |_| Event::None);
                caps.render.render();
            }
            Event::Get => {
                if let Some(_fact) = &model.cat_fact {
                    caps.render.render()
                } else {
                    self.update(Event::Fetch, model, caps)
                }
            }
            Event::Fetch => {
                model.cat_image = Some(CatImage::default());

                caps.http
                    .get(FACT_API_URL)
                    .expect_json()
                    .send(Event::SetFact);

                caps.http
                    .get(IMAGE_API_URL)
                    .expect_json()
                    .send(Event::SetImage);

                caps.render.render();
            }
            Event::SetFact(Ok(mut response)) => {
                model.cat_fact = Some(response.take_body().unwrap());

                caps.time.now(Event::CurrentTime);
            }
            Event::SetImage(Ok(mut response)) => {
                model.cat_image = Some(response.take_body().unwrap());

                let bytes = serde_json::to_vec(&model).unwrap();
                caps.key_value.set(KEY.to_string(), bytes, |_| Event::None);

                caps.render.render();
            }
            Event::SetFact(Err(_)) | Event::SetImage(Err(_)) => {
                // TODO: Display an error
            }
            Event::CurrentTime(TimeResponse::Now(instant)) => {
                let time: DateTime<Utc> = instant.try_into().unwrap();
                model.time = Some(time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));

                let bytes = serde_json::to_vec(&model).unwrap();
                caps.key_value.set(KEY.to_string(), bytes, |_| Event::None);

                caps.render.render();
            }
            Event::CurrentTime(_) => panic!("Unexpected time response"),
            Event::Restore => {
                caps.key_value.get(KEY.to_string(), Event::SetState);
            }
            Event::SetState(Ok(Some(value))) => {
                if let Ok(m) = serde_json::from_slice::<Model>(&value) {
                    *model = m;
                    caps.render.render();
                };
            }
            Event::SetState(Ok(None)) => {
                // no state to restore
            }
            Event::SetState(Err(_)) => {
                // handle error
            }
            Event::None => {}
        }
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

#[cfg(test)]
mod tests {
    use crux_core::testing::AppTester;
    use crux_http::{
        protocol::{HttpRequest, HttpResponse, HttpResult},
        testing::ResponseBuilder,
    };
    use crux_kv::{value::Value, KeyValueOperation, KeyValueResponse, KeyValueResult};
    use crux_time::Instant;

    use super::*;

    #[test]
    fn fetch_results_in_set_fact_and_set_image() {
        let app = AppTester::<CatFacts, _>::default();
        let mut model = Model::default();

        // send fetch event to app
        let (mut http_effects, mut render_effects) = app
            .update(Event::Fetch, &mut model)
            .take_effects_partitioned_by(Effect::is_http);

        // receive render effect
        render_effects.pop_front().unwrap().expect_render();

        // receive two HTTP effects, one to fetch the fact and one to fetch the image
        // we'll handle the fact request first
        let request = &mut http_effects.pop_front().unwrap().expect_http();
        assert_eq!(request.operation, HttpRequest::get(FACT_API_URL).build());

        let a_fact = CatFact {
            fact: "cats are good".to_string(),
            length: 13,
        };

        // resolve the request with a simulated response from the web API
        let event = app
            .resolve(
                request,
                HttpResult::Ok(
                    HttpResponse::ok()
                        .body(r#"{ "fact": "cats are good", "length": 13 }"#)
                        .build(),
                ),
            )
            .expect("should resolve successfully")
            .expect_one_event();

        // check that the app emitted an (internal) event to update the model
        assert_eq!(
            event,
            Event::SetFact(Ok(ResponseBuilder::ok().body(a_fact.clone()).build()))
        );

        // Setting the fact should trigger a time event
        let mut time_events = app.update(event, &mut model).take_effects(|e| e.is_time());

        let request = &mut time_events.pop_front().unwrap().expect_time();

        let response = TimeResponse::Now(Instant::new(0, 0).unwrap());
        let event = app
            .resolve(request, response)
            .expect("should resolve successfully")
            .expect_one_event();

        assert_eq!(event, Event::CurrentTime(response));

        // update the app with the current time event
        // and check that we get a key value set event and a render event
        let (mut key_value_effects, mut render_effects) = app
            .update(event, &mut model)
            .take_effects_partitioned_by(Effect::is_key_value);
        render_effects.pop_front().unwrap().expect_render();

        let request = &mut key_value_effects.pop_front().unwrap().expect_key_value();
        assert_eq!(
            request.operation,
            KeyValueOperation::Set {
                key: KEY.to_string(),
                value: serde_json::to_vec(&model).unwrap()
            }
        );

        let _updated = app.resolve_to_event_then_update(
            request,
            KeyValueResult::Ok {
                response: KeyValueResponse::Set {
                    previous: Value::None,
                },
            },
            &mut model,
        );

        // Now we'll handle the image
        let request = &mut http_effects.pop_front().unwrap().expect_http();
        assert_eq!(request.operation, HttpRequest::get(IMAGE_API_URL).build());

        let an_image = CatImage {
            href: "image_url".to_string(),
        };

        let response = HttpResult::Ok(HttpResponse::ok().body(r#"{"href":"image_url"}"#).build());
        let _updated = app.resolve_to_event_then_update(request, response, &mut model);

        assert_eq!(model.cat_fact, Some(a_fact));
        assert_eq!(model.cat_image, Some(an_image));
    }
}
