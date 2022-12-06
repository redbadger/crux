use crux_core::{render::Render, App};
use crux_platform::{Platform as PlatformCap, PlatformResponse};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct Platform {}

#[derive(Default, Serialize, Deserialize)]
pub struct Model {
    pub platform: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum PlatformEvent {
    Get,
    Set(PlatformResponse),
}

pub struct PlatformCapabilities {
    pub platform: PlatformCap<PlatformEvent>,
    pub render: Render<PlatformEvent>,
}

impl App for Platform {
    type Event = PlatformEvent;
    type Model = Model;
    type ViewModel = Model;
    type Capabilities = PlatformCapabilities;

    fn update(&self, msg: PlatformEvent, model: &mut Model, caps: &PlatformCapabilities) {
        match msg {
            PlatformEvent::Get => caps.platform.get(PlatformEvent::Set),
            PlatformEvent::Set(platform) => {
                model.platform = platform.0;
                caps.render.render()
            }
        }
    }

    fn view(&self, model: &Model) -> Model {
        Model {
            platform: model.platform.clone(),
        }
    }
}
