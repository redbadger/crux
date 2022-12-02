use crux_core::{render::Render, App, Capabilities, Command, Commander};
use crux_platform::{Platform as PlatformCap, PlatformResponse};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use crate::effect::CatFactCapabilities;

#[derive(Default)]
pub struct Platform<Ef> {
    _marker: PhantomData<fn() -> Ef>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Model {
    pub platform: String,
}

#[derive(Serialize, Deserialize)]
pub enum PlatformEvent {
    Get,
    Set(PlatformResponse),
}

// A thought: arguably this doesn't even need to be an `App` since nothing generic is driving it...
impl<Ef> App<Ef> for Platform<Ef>
where
    Ef: Serialize + Clone + Default,
{
    type Event = PlatformEvent;
    type Model = Model;
    type ViewModel = Model;
    type Capabilities = CatFactCapabilities;

    fn update(&self, msg: PlatformEvent, model: &mut Model, caps: &CatFactCapabilities) {
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
