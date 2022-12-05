use crux_core::{render::Render, App, Capabilities, Command};
use crux_platform::{Platform as PlatformCap, PlatformResponse};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

pub struct Platform<Ef, Caps> {
    _marker: PhantomData<fn() -> (Ef, Caps)>,
}

impl<Ef, Caps> Default for Platform<Ef, Caps> {
    fn default() -> Self {
        Self {
            _marker: Default::default(),
        }
    }
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

impl<Ef, Caps> App<Ef, Caps> for Platform<Ef, Caps>
where
    Ef: Serialize + Clone,
    Caps: Capabilities<PlatformCap<Ef>> + Capabilities<Render<Ef>>,
{
    type Event = PlatformEvent;
    type Model = Model;
    type ViewModel = Model;

    fn update(
        &self,
        msg: PlatformEvent,
        model: &mut Model,
        caps: &Caps,
    ) -> Vec<Command<Ef, PlatformEvent>> {
        let platform: &PlatformCap<_> = caps.get();
        let render: &Render<_> = caps.get();

        match msg {
            PlatformEvent::Get => {
                vec![platform.get(PlatformEvent::Set)]
            }
            PlatformEvent::Set(platform) => {
                model.platform = platform.0;
                vec![render.render()]
            }
        }
    }

    fn view(&self, model: &<Self as App<Ef, Caps>>::Model) -> <Self as App<Ef, Caps>>::ViewModel {
        Model {
            platform: model.platform.clone(),
        }
    }
}
