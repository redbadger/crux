use crux_core::{platform, App, Command};
use serde::{Deserialize, Serialize};

use crate::effect::{Capabilities, Effect};

#[derive(Default)]
pub struct Platform {
    capabilities: Capabilities,
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

impl App for Platform {
    type Event = Event;
    type Effect = Effect;
    type Model = Model;
    type ViewModel = Model;

    fn update(&self, msg: Event, model: &mut Model) -> Vec<Command<Effect, Event>> {
        match msg {
            Event::Get => vec![self.capabilities.platform.get(Event::Set)],
            Event::Set(platform) => {
                model.platform = platform.0;
                vec![self.capabilities.render.render()]
            }
        }
    }

    fn view(&self, model: &<Self as App>::Model) -> <Self as App>::ViewModel {
        Model {
            platform: model.platform.clone(),
        }
    }
}
