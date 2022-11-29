use crux_core::{platform, render, App, Capabilities, Capability, Command};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

pub struct Platform<Ef, Caps>
where
    Caps: Default,
{
    capabilities: Caps,
    _marker: PhantomData<Ef>,
}

impl<Ef, Caps> Default for Platform<Ef, Caps>
where
    Caps: Default,
{
    fn default() -> Self {
        Self {
            capabilities: Default::default(),
            _marker: PhantomData,
        }
    }
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
    Ef: Serialize + Clone,
    Caps: Default + Capabilities<platform::Platform<Ef>> + Capabilities<render::Render<Ef>>,
{
    type Event = Event;
    type Model = Model;
    type ViewModel = Model;

    fn update(&self, msg: Event, model: &mut Model) -> Vec<Command<Ef, Event>> {
        match msg {
            Event::Get => vec![self.capability::<platform::Platform<_>>().get(Event::Set)],
            Event::Set(platform) => {
                model.platform = platform.0;
                vec![self.capability::<render::Render<_>>().render()]
            }
        }
    }

    fn view(&self, model: &<Self as App<Ef, Caps>>::Model) -> <Self as App<Ef, Caps>>::ViewModel {
        Model {
            platform: model.platform.clone(),
        }
    }
}

impl<Ef, Caps> Platform<Ef, Caps>
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
