pub use rmm::*;
use serde::{Deserialize, Serialize};
use shared_types::{CatImage, Msg, ViewModel};

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

// Expose the Core for other platforms;
pub type Core = AppCore<CatFacts>;

#[derive(Default)]
pub struct CatFacts {}

#[derive(Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Model {
    cat_fact: Option<CatFact>,
    cat_image: Option<CatImage>,
    platform: String,
    time: Option<String>,
}

impl From<&Model> for ViewModel {
    fn from(model: &Model) -> Self {
        let fact = match (&model.cat_fact, &model.time) {
            (Some(fact), Some(time)) => format!("Fact from {}: {}", time, fact.format()),
            _ => "No fact".to_string(),
        };

        ViewModel {
            platform: format!("Hello {}", model.platform),
            fact,
            image: model.cat_image.clone(),
        }
    }
}

impl App for CatFacts {
    type Msg = Msg;
    type Model = Model;
    type ViewModel = ViewModel;

    fn update(&self, msg: Msg, model: &mut Model, cmd: &Cmd<Msg>) -> Vec<Request> {
        match msg {
            Msg::GetPlatform => vec![cmd.platform.get(|platform| Msg::SetPlatform { platform })],
            Msg::SetPlatform { platform } => {
                model.platform = platform;
                vec![Request::render()]
            }
            Msg::Clear => {
                model.cat_fact = None;
                model.cat_image = None;
                let bytes = serde_json::to_vec(&model).unwrap();

                vec![
                    cmd.key_value_write
                        .write("state".to_string(), bytes, |_| Msg::None),
                    Request::render(),
                ]
            }
            Msg::Get => {
                if let Some(_fact) = &model.cat_fact {
                    vec![Request::render()]
                } else {
                    model.cat_image = Some(CatImage::default());

                    vec![
                        cmd.http
                            .get(FACT_API_URL.to_owned(), |bytes| Msg::SetFact { bytes }),
                        cmd.http
                            .get(IMAGE_API_URL.to_string(), |bytes| Msg::SetImage { bytes }),
                        Request::render(),
                    ]
                }
            }
            Msg::Fetch => {
                model.cat_image = Some(CatImage::default());

                vec![
                    cmd.http
                        .get(FACT_API_URL.to_owned(), |bytes| Msg::SetFact { bytes }),
                    cmd.http
                        .get(IMAGE_API_URL.to_string(), |bytes| Msg::SetImage { bytes }),
                    Request::render(),
                ]
            }
            Msg::SetFact { bytes } => {
                let fact = serde_json::from_slice::<CatFact>(&bytes).unwrap();
                model.cat_fact = Some(fact);

                let bytes = serde_json::to_vec(&model).unwrap();

                vec![
                    cmd.key_value_write
                        .write("state".to_string(), bytes, |_| Msg::None),
                    cmd.time.get(|iso_time| Msg::CurrentTime { iso_time }),
                ]
            }
            Msg::CurrentTime { iso_time } => {
                model.time = Some(iso_time);
                let bytes = serde_json::to_vec(&model).unwrap();

                vec![
                    cmd.key_value_write
                        .write("state".to_string(), bytes, |_| Msg::None),
                    Request::render(),
                ]
            }
            Msg::SetImage { bytes } => {
                let image = serde_json::from_slice::<CatImage>(&bytes).unwrap();
                model.cat_image = Some(image);

                let bytes = serde_json::to_vec(&model).unwrap();

                vec![
                    cmd.key_value_write
                        .write("state".to_string(), bytes, |_| Msg::None),
                    Request::render(),
                ]
            }
            Msg::Restore => {
                vec![cmd
                    .key_value_read
                    .read("state".to_string(), |bytes| Msg::SetState { bytes })]
            }
            Msg::SetState { bytes } => {
                if let Some(bytes) = bytes {
                    if let Ok(m) = serde_json::from_slice::<Model>(&bytes) {
                        *model = m
                    };
                }

                vec![Request::render()]
            }
            Msg::None => vec![],
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        model.into()
    }
}

uniffi_macros::include_scaffolding!("shared");
