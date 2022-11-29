use crux_core::{
    http::{self, Http},
    key_value::{self, KeyValue},
    platform::Platform,
    render::Render,
    time::{self, Time},
    Capabilities, Capability,
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
    Platform(platform::Event),
    Clear,
    Get,
    Fetch,
    Restore,                       // restore state
    SetState(key_value::Response), // receive the data to restore state with
    SetFact(http::Response),
    SetImage(http::Response),
    CurrentTime(time::Response),
}

pub struct CatFacts<Ef, Caps>
where
    Caps: Default,
{
    capabilities: Caps,
    platform: platform::Platform<Ef, Caps>,
}

impl<Ef, Caps> Default for CatFacts<Ef, Caps>
where
    Caps: Default,
{
    fn default() -> Self {
        Self {
            capabilities: Default::default(),
            platform: Default::default(),
        }
    }
}

impl<Ef, Caps> App<Ef, Caps> for CatFacts<Ef, Caps>
where
    Ef: Serialize + Clone,
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

    fn update(&self, msg: Event, model: &mut Model) -> Vec<Command<Ef, Event>> {
        match msg {
            Event::GetPlatform => Command::lift(
                self.platform
                    .update(platform::Event::Get, &mut model.platform),
                Event::Platform,
            ),
            Event::Platform(msg) => Command::lift(
                self.platform.update(msg, &mut model.platform),
                Event::Platform,
            ),
            Event::Clear => {
                model.cat_fact = None;
                model.cat_image = None;
                let bytes = serde_json::to_vec(&model).unwrap();

                vec![
                    self.capability::<key_value::KeyValue<_>>()
                        .write("state", bytes, |_| Event::None),
                    self.capability::<Render<_>>().render(),
                ]
            }
            Event::Get => {
                if let Some(_fact) = &model.cat_fact {
                    vec![self.capability::<Render<_>>().render()]
                } else {
                    model.cat_image = Some(CatImage::default());

                    vec![
                        self.capability::<http::Http<_>>()
                            .get(FACT_API_URL, Event::SetFact),
                        self.capability::<http::Http<_>>()
                            .get(IMAGE_API_URL, Event::SetImage),
                        self.capability::<Render<_>>().render(),
                    ]
                }
            }
            Event::Fetch => {
                model.cat_image = Some(CatImage::default());

                vec![
                    self.capability::<http::Http<_>>()
                        .get(FACT_API_URL, Event::SetFact),
                    self.capability::<http::Http<_>>()
                        .get(IMAGE_API_URL, Event::SetImage),
                    self.capability::<Render<_>>().render(),
                ]
            }
            Event::SetFact(http::Response { body, status: _ }) => {
                // TODO check status
                let fact = serde_json::from_slice::<CatFact>(&body).unwrap();
                model.cat_fact = Some(fact);

                let bytes = serde_json::to_vec(&model).unwrap();

                vec![
                    self.capability::<key_value::KeyValue<_>>()
                        .write("state", bytes, |_| Event::None),
                    self.capability::<time::Time<_>>().get(Event::CurrentTime),
                ]
            }
            Event::CurrentTime(iso_time) => {
                model.time = Some(iso_time.0);
                let bytes = serde_json::to_vec(&model).unwrap();

                vec![
                    self.capability::<key_value::KeyValue<_>>()
                        .write("state", bytes, |_| Event::None),
                    self.capability::<Render<_>>().render(),
                ]
            }
            Event::SetImage(http::Response { body, status: _ }) => {
                // TODO check status
                let image = serde_json::from_slice::<CatImage>(&body).unwrap();
                model.cat_image = Some(image);

                let bytes = serde_json::to_vec(&model).unwrap();

                vec![
                    self.capability::<key_value::KeyValue<_>>()
                        .write("state", bytes, |_| Event::None),
                    self.capability::<Render<_>>().render(),
                ]
            }
            Event::Restore => {
                vec![self
                    .capability::<key_value::KeyValue<_>>()
                    .read("state", Event::SetState)]
            }
            Event::SetState(response) => {
                if let key_value::Response::Read(Some(bytes)) = response {
                    if let Ok(m) = serde_json::from_slice::<Model>(&bytes) {
                        *model = m
                    };
                }

                vec![self.capability::<Render<_>>().render()]
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

        let platform = self.platform.view(&model.platform).platform;

        ViewModel {
            platform: format!("Hello {}", platform),
            fact,
            image: model.cat_image.clone(),
        }
    }
}

impl<Ef, Caps> CatFacts<Ef, Caps>
where
    Ef: Serialize + Clone,
    Caps: Default,
{
    fn capability<C>(&self) -> &C
    where
        C: Capability,
        Caps: Capabilities<C>,
    {
        <Caps as crux_core::Capabilities<C>>::get(&self.capabilities)
    }
}
