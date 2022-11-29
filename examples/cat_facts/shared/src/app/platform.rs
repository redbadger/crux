use std::marker::PhantomData;

use crux_core::{
    platform::{self, Platform as PlatformCap},
    render::Render,
    App, Capabilities, Command,
};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct Platform<Ef, Caps> {
    _marker: PhantomData<fn() -> (Ef, Caps)>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Model {
    pub platform: String,
}

#[derive(Serialize, Deserialize)]
pub enum Event {
    Get,
    Set(platform::Response),
}

impl<Ef, Caps> App<Ef, Caps> for Platform<Ef, Caps>
where
    Ef: Serialize + Clone + Default,
    Caps: Default + Capabilities<PlatformCap<Ef>> + Capabilities<Render<Ef>>,
{
    type Event = Event;
    type Model = Model;
    type ViewModel = Model;

    fn update(&self, msg: Event, model: &mut Model, caps: &Caps) -> Vec<Command<Ef, Event>> {
        let platform = <Caps as crux_core::Capabilities<PlatformCap<_>>>::get(caps);
        let render = <Caps as crux_core::Capabilities<Render<_>>>::get(caps);

        match msg {
            Event::Get => {
                vec![platform.get(Event::Set)]
            }
            Event::Set(platform) => {
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
