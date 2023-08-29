pub mod platform;

use serde::{Deserialize, Serialize};

pub use crux_core::App;
use crux_core::{render::Render, Capability};
use crux_http::Http;
use crux_kv::{KeyValue, KeyValueOutput};
use crux_macros::Effect;
use crux_platform::Platform;
use crux_time::{Time, TimeResponse};

use platform::Capabilities;

const CAT_LOADING_URL: &str = "https://c.tenor.com/qACzaJ1EBVYAAAAd/tenor.gif";
const FACT_API_URL: &str = "https://catfact.ninja/fact";
const IMAGE_API_URL: &str = "https://crux-counter.fly.dev/cat";

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
    Platform(platform::Event),
    SetState(KeyValueOutput), // receive the data to restore state with
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

#[cfg_attr(feature = "typegen", derive(crux_macros::Export))]
#[derive(Effect)]
#[effect(app = "CatFacts")]
pub struct CatFactCapabilities {
    pub http: Http<Event>,
    pub key_value: KeyValue<Event>,
    pub platform: Platform<Event>,
    pub render: Render<Event>,
    pub time: Time<Event>,
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

                caps.key_value.write("state", bytes, |_| Event::None);
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
                // TODO check status
                model.cat_fact = Some(response.take_body().unwrap());

                let bytes = serde_json::to_vec(&model).unwrap();
                caps.key_value.write("state", bytes, |_| Event::None);

                caps.time.get(Event::CurrentTime);
            }
            Event::SetImage(Ok(mut response)) => {
                if response.status().is_success() {
                    model.cat_image = Some(response.take_body().unwrap());

                    let bytes = serde_json::to_vec(&model).unwrap();
                    caps.key_value.write("state", bytes, |_| Event::None);

                    caps.render.render();
                } else {
                    self.update(
                        Event::SetImage(Err(crux_http::Error::new(
                            Some(response.status()),
                            "fetching cat image failed",
                        ))),
                        model,
                        caps,
                    );
                }
            }
            Event::SetFact(Err(_)) | Event::SetImage(Err(_)) => {
                // TODO: Display an error or something?
            }
            Event::CurrentTime(iso_time) => {
                model.time = Some(iso_time.0);
                let bytes = serde_json::to_vec(&model).unwrap();
                caps.key_value.write("state", bytes, |_| Event::None);

                caps.render.render();
            }
            Event::Restore => {
                caps.key_value.read("state", Event::SetState);
            }
            Event::SetState(response) => {
                if let KeyValueOutput::Read(Some(bytes)) = response {
                    if let Ok(m) = serde_json::from_slice::<Model>(&bytes) {
                        *model = m
                    };
                }

                caps.render.render()
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
    use assert_let_bind::assert_let;
    use crux_core::testing::AppTester;
    use crux_http::{
        protocol::{HttpRequest, HttpResponse},
        testing::ResponseBuilder,
    };

    use crate::Effect;

    use super::*;

    #[test]
    fn fetch_sends_some_requests() {
        let app = AppTester::<CatFacts, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::Fetch, &mut model);

        assert_let!(Effect::Http(request), update.effects().next().unwrap());
        let actual = &request.operation;
        let expected = &HttpRequest::get(FACT_API_URL).build();

        assert_eq!(actual, expected);
    }

    #[test]
    fn fetch_results_in_set_fact_and_set_image() {
        let app = AppTester::<CatFacts, _>::default();
        let mut model = Model::default();

        let mut update = app.update(Event::Fetch, &mut model);
        let mut effects = update.effects_mut();

        assert_let!(Effect::Http(request), effects.next().unwrap());
        let actual = &request.operation;
        let expected = &HttpRequest::get(FACT_API_URL).build();
        assert_eq!(actual, expected);

        let a_fact = CatFact {
            fact: "cats are good".to_string(),
            length: 13,
        };

        let response = HttpResponse::ok()
            .body(r#"{ "fact": "cats are good", "length": 13 }"#)
            .build();
        let update = app
            .resolve(request, response)
            .expect("should resolve successfully");

        let expected_response = ResponseBuilder::ok().body(a_fact.clone()).build();
        assert_eq!(update.events, vec![Event::SetFact(Ok(expected_response))]);

        for event in update.events {
            app.update(event, &mut model);
        }

        assert_let!(Effect::Http(request), effects.next().unwrap());
        let actual = &request.operation;
        let expected = &HttpRequest::get(IMAGE_API_URL).build();
        assert_eq!(actual, expected);

        let a_image = CatImage {
            href: "image_url".to_string(),
        };

        let response = HttpResponse::ok().body(r#"{"href":"image_url"}"#).build();
        let update = app
            .resolve(request, response)
            .expect("should resolve successfully");
        for event in update.events {
            app.update(event, &mut model);
        }

        assert_eq!(model.cat_fact, Some(a_fact));
        assert_eq!(model.cat_image, Some(a_image));
    }
}
