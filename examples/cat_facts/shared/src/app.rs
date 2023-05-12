pub mod platform;

use serde::{Deserialize, Serialize};

pub use crux_core::App;
use crux_core::{render::Render, Capability};
use crux_http::Http;
use crux_kv::{KeyValue, KeyValueOutput};
use crux_macros::Effect;
use crux_platform::Platform;
use crux_time::{Time, TimeResponse};

use platform::{PlatformCapabilities, PlatformEvent};

const CAT_LOADING_URL: &str = "https://c.tenor.com/qACzaJ1EBVYAAAAd/tenor.gif";
const FACT_API_URL: &str = "https://catfact.ninja/fact";
const IMAGE_API_URL: &str = "https://aws.random.cat/meow";

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

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct CatImage {
    pub file: String,
}

impl Default for CatImage {
    fn default() -> Self {
        Self {
            file: CAT_LOADING_URL.to_string(),
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
    Platform(PlatformEvent),
    SetState(KeyValueOutput), // receive the data to restore state with
    CurrentTime(TimeResponse),
    #[serde(skip)]
    SetFact(crux_http::Result<crux_http::Response<CatFact>>),
    #[serde(skip)]
    SetImage(crux_http::Result<crux_http::Response<CatImage>>),
}

#[derive(Default)]
pub struct CatFacts {
    platform: platform::Platform,
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
impl From<&CatFactCapabilities> for PlatformCapabilities {
    fn from(incoming: &CatFactCapabilities) -> Self {
        PlatformCapabilities {
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
                    .update(PlatformEvent::Get, &mut model.platform, &caps.into())
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
                // TODO check status
                model.cat_image = Some(response.take_body().unwrap());

                let bytes = serde_json::to_vec(&model).unwrap();
                caps.key_value.write("state", bytes, |_| Event::None);

                caps.render.render();
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
            <platform::Platform as crux_core::App>::view(&self.platform, &model.platform).platform;

        ViewModel {
            platform: format!("Hello {platform}"),
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

        assert_let!(Effect::Http(request), &update.effects[0]);
        let actual = &request.operation;
        let expected = &HttpRequest {
            method: "GET".into(),
            url: FACT_API_URL.into(),
            headers: vec![],
            body: vec![],
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn fact_response_results_in_set_fact() {
        let app = AppTester::<CatFacts, _>::default();
        let mut model = Model::default();

        let mut update = app.update(Event::Fetch, &mut model);

        assert_let!(Effect::Http(request), &mut update.effects[0]);
        let actual = &request.operation;
        let expected = &HttpRequest {
            method: "GET".into(),
            url: FACT_API_URL.into(),
            headers: vec![],
            body: vec![],
        };
        assert_eq!(actual, expected);

        let a_fact = CatFact {
            fact: "cats are good".to_string(),
            length: 13,
        };

        let response = HttpResponse {
            status: 200,
            body: serde_json::to_vec(&a_fact).unwrap(),
        };
        let update = app
            .resolve(request, response)
            .expect("should resolve successfully");

        let expected_response = ResponseBuilder::ok().body(a_fact).build();
        assert_eq!(update.events, vec![Event::SetFact(Ok(expected_response))])
    }
}
