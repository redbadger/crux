use self::platform::PlatformEvent;
use crux_core::{
    http::{Http, HttpResponse},
    key_value::{KeyValue, KeyValueResponse},
    platform::Platform,
    render::Render,
    time::{Time, TimeResponse},
    Capabilities,
};
pub use crux_core::{App, Command};
use serde::{Deserialize, Serialize};

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
pub struct CatFacts<Ef, Caps> {
    platform: platform::Platform<Ef, Caps>,
}

impl<Ef, Caps> App<Ef, Caps> for CatFacts<Ef, Caps>
where
    Ef: Serialize + Clone + Default,
    Caps: Default
        + Capabilities<Http<Ef>>
        + Capabilities<KeyValue<Ef>>
        + Capabilities<Render<Ef>>
        + Capabilities<Time<Ef>>
        + Capabilities<Platform<Ef>>,
{
    type Model = Model;
    type Event = Event;
    type ViewModel = ViewModel;

    fn update(&self, msg: Event, model: &mut Model, caps: &Caps) -> Vec<Command<Ef, Event>> {
        let http = <Caps as crux_core::Capabilities<Http<_>>>::get(caps);
        let key_value = <Caps as crux_core::Capabilities<KeyValue<_>>>::get(caps);
        let render = <Caps as crux_core::Capabilities<Render<_>>>::get(caps);
        let time = <Caps as crux_core::Capabilities<Time<_>>>::get(caps);

        match msg {
            Event::GetPlatform => Command::lift(
                self.platform
                    .update(PlatformEvent::Get, &mut model.platform, caps),
                Event::Platform,
            ),
            Event::Platform(msg) => Command::lift(
                self.platform.update(msg, &mut model.platform, caps),
                Event::Platform,
            ),
            Event::Clear => {
                model.cat_fact = None;
                model.cat_image = None;
                let bytes = serde_json::to_vec(&model).unwrap();

                vec![
                    key_value.write("state", bytes, |_| Event::None),
                    render.render(),
                ]
            }
            Event::Get => {
                if let Some(_fact) = &model.cat_fact {
                    vec![render.render()]
                } else {
                    self.update(Event::Fetch, model, caps)
                }
            }
            Event::Fetch => {
                model.cat_image = Some(CatImage::default());

                vec![
                    http.get(FACT_API_URL, Event::SetFact),
                    http.get(IMAGE_API_URL, Event::SetImage),
                    render.render(),
                ]
            }
            Event::SetFact(HttpResponse { body, status: _ }) => {
                // TODO check status
                let fact = serde_json::from_slice::<CatFact>(&body).unwrap();
                model.cat_fact = Some(fact);

                let bytes = serde_json::to_vec(&model).unwrap();

                vec![
                    key_value.write("state", bytes, |_| Event::None),
                    time.get(Event::CurrentTime),
                ]
            }
            Event::CurrentTime(iso_time) => {
                model.time = Some(iso_time.0);
                let bytes = serde_json::to_vec(&model).unwrap();

                vec![
                    key_value.write("state", bytes, |_| Event::None),
                    render.render(),
                ]
            }
            Event::SetImage(HttpResponse { body, status: _ }) => {
                // TODO check status
                let image = serde_json::from_slice::<CatImage>(&body).unwrap();
                model.cat_image = Some(image);

                let bytes = serde_json::to_vec(&model).unwrap();

                vec![
                    key_value.write("state", bytes, |_| Event::None),
                    render.render(),
                ]
            }
            Event::Restore => {
                vec![key_value.read("state", Event::SetState)]
            }
            Event::SetState(response) => {
                if let KeyValueResponse::Read(Some(bytes)) = response {
                    if let Ok(m) = serde_json::from_slice::<Model>(&bytes) {
                        *model = m
                    };
                }

                vec![render.render()]
            }
            Event::None => vec![],
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        let fact = match (&model.cat_fact, &model.time) {
            (Some(fact), Some(time)) => format!("Fact from {}: {}", time, fact.format()),
            (Some(fact), _) => fact.format(),
            _ => "No fact".to_string(),
        };

        let platform = <platform::Platform<Ef, Caps> as crux_core::App<Ef, Caps>>::view(
            &self.platform,
            &model.platform,
        )
        .platform;

        ViewModel {
            platform: format!("Hello {}", platform),
            fact,
            image: model.cat_image.clone(),
        }
    }
}
