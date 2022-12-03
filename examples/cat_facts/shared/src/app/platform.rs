use crux_core::{render::Render, App, Capabilities, Command};
use crux_platform::{Platform as PlatformCap, PlatformResponse};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use crate::effect::CatFactCapabilities;

#[derive(Default)]
pub struct Platform {}

#[derive(Default, Serialize, Deserialize)]
pub struct Model {
    pub platform: String,
}

#[derive(Serialize, Deserialize)]
pub enum PlatformEvent {
    Get,
    Set(PlatformResponse),
}

pub struct PlatformCapabilities {
    pub http: crux_http::Http<PlatformEvent>,
}

// A thought: arguably this doesn't even need to be an `App` since nothing generic is driving it...
impl App for Platform {
    type Event = PlatformEvent;
    type Model = Model;
    type ViewModel = Model;
    type Capabilities = PlatformCapabilities;

    fn update(&self, msg: PlatformEvent, model: &mut Model, caps: &PlatformCapabilities) {

        // let platform: &PlatformCap<_> = caps.get();
        // let render: &Render<_> = caps.get();

        /*
        match msg {
            PlatformEvent::Get => platform.get(PlatformEvent::Set),
            PlatformEvent::Set(platform) => {
                model.platform = platform.0;
                render.render()
            }
        } */
    }

    fn view(&self, model: &Model) -> Model {
        Model {
            platform: model.platform.clone(),
        }
    }
}
