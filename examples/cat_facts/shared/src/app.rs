use crate::effect::CatFactCapabilities;

use self::platform::PlatformEvent;
use crux_core::{render::Render, Capabilities};
pub use crux_core::{App, Command};
use crux_http::{Http, HttpResponse};
use crux_kv::{KeyValue, KeyValueResponse};
use crux_platform::Platform;
use crux_time::{Time, TimeResponse};
use serde::{Deserialize, Serialize};
use url::Url;

pub mod platform;

const CAT_LOADING_URL: &str = "https://c.tenor.com/qACzaJ1EBVYAAAAd/tenor.gif";
const FACT_API_URL: &str = "https://catfact.ninja/fact";
const IMAGE_API_URL: &str = "https://aws.random.cat/meow";

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
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

#[derive(Serialize, Deserialize)]
pub enum Event {
    None,
    GetPlatform,
    Platform(PlatformEvent),
    Clear,
    Get,
    Fetch,
    Restore,                    // restore state
    SetState(KeyValueResponse), // receive the data to restore state with
    SetFact(HttpResponse),
    SetImage(HttpResponse),
    CurrentTime(TimeResponse),
}

#[derive(Default)]
pub struct CatFacts {
    platform: platform::Platform,
}

// TODO: also get rid of Ef from here if possible.
impl App for CatFacts {
    type Model = Model;
    type Event = Event;
    type ViewModel = ViewModel;
    type Capabilities = CatFactCapabilities;

    fn update(&self, msg: Event, model: &mut Model, caps: &CatFactCapabilities) {
        // TOOD: ok, so I think the reason I'm struggling here is that we have `Caps` and we have `Commander` and they're separate.
        // If I merge them does this become easier?  Then we don't need the `Ef` "binding" type-param here.
        // We just need `Caps` to hide the `Ef` type somehow?
        // So we could make a

        // let key_value: &KeyValue<_> = caps.get();
        // let render: &Render<_> = caps.get();
        // let time: &Time<_> = caps.get();

        // Ok, so some thoughts:
        // 1. Don't return a Vec<Command<..>>.
        //    - I know from experience it gets a tiny bit awkward when you need to
        //      do things conditionally.
        //    - I also think it means you end up with far more generic params than you
        //      need.
        // 2. Use channels/vecs/whatever instead probably?
        //    - The eff parameter can be erased from this struct entirely then.
        //    -

        match msg {
            Event::GetPlatform => {
                todo!()
            } /*Command::lift(
            self.platform
            .update(PlatformEvent::Get, &mut model.platform, caps, commander),
            Event::Platform,
            ),*/
            Event::Platform(msg) => {
                todo!()
            } /*Command::lift(
            self.platform
            .update(msg, &mut model.platform, caps, commander),
            Event::Platform,
            ),*/
            Event::Clear => {
                model.cat_fact = None;
                model.cat_image = None;
                let bytes = serde_json::to_vec(&model).unwrap();

                /*
                vec![
                    key_value.write("state", bytes, |_| Event::None),
                    render.render(),
                ]); */
            }
            Event::Get => {
                if let Some(_fact) = &model.cat_fact {
                    // commander.send_command(render.render())
                } else {
                    self.update(Event::Fetch, model, caps)
                }
            }
            Event::Fetch => {
                model.cat_image = Some(CatImage::default());

                caps.http
                    .get(Url::parse(FACT_API_URL).unwrap(), Event::SetFact);
                caps.http
                    .get(Url::parse(IMAGE_API_URL).unwrap(), Event::SetImage);
                // render.render(),
            }
            Event::SetFact(HttpResponse { body, status: _ }) => {
                // TODO check status
                let fact = serde_json::from_slice::<CatFact>(&body).unwrap();
                model.cat_fact = Some(fact);

                let bytes = serde_json::to_vec(&model).unwrap();

                /*vec![
                    key_value.write("state", bytes, |_| Event::None),
                    time.get(Event::CurrentTime),
                ]*/
            }
            Event::CurrentTime(iso_time) => {
                model.time = Some(iso_time.0);
                let bytes = serde_json::to_vec(&model).unwrap();

                /*
                vec![
                    key_value.write("state", bytes, |_| Event::None),
                    render.render(),
                ] */
            }
            Event::SetImage(HttpResponse { body, status: _ }) => {
                // TODO check status
                let image = serde_json::from_slice::<CatImage>(&body).unwrap();
                model.cat_image = Some(image);

                let bytes = serde_json::to_vec(&model).unwrap();

                /*
                vec![
                    key_value.write("state", bytes, |_| Event::None),
                    render.render(),
                ] */
            }
            Event::Restore => {} /*key_value.read("state", Event::SetState))*/
            Event::SetState(response) => {
                /*
                if let KeyValueResponse::Read(Some(bytes)) = response {
                    if let Ok(m) = serde_json::from_slice::<Model>(&bytes) {
                        *model = m
                    };
                }

                render.render()
                */
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
            platform: format!("Hello {}", platform),
            fact,
            image: model.cat_image.clone(),
        }
    }
}
