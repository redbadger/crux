use crux_core::{
    App, Command,
    macros::effect,
    render::{self, RenderOperation},
};
use crux_platform::{PlatformRequest, PlatformResponse};
use facet::Facet;
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct Platform {}

#[derive(Default, Serialize, Deserialize)]
pub struct Model {
    pub(crate) platform: String,
}

#[derive(Facet, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub enum Event {
    Get,
    Set(PlatformResponse),
}

#[effect]
pub enum Effect {
    Platform(PlatformRequest),
    Render(RenderOperation),
}

impl App for Platform {
    type Event = Event;
    type Model = Model;
    type ViewModel = Model;
    type Effect = Effect;

    fn update(&self, msg: Event, model: &mut Model) -> Command<Effect, Event> {
        match msg {
            Event::Get => crux_platform::Platform::get().then_send(Event::Set),
            Event::Set(PlatformResponse(platform)) => {
                model.platform = platform;
                render::render()
            }
        }
    }

    fn view(&self, model: &Model) -> Model {
        Model {
            platform: format!("Hello {platform}", platform = model.platform),
        }
    }
}

#[cfg(test)]
mod tests {
    use crux_platform::PlatformResponse;

    use super::*;

    #[test]
    fn get_platform() {
        let app = Platform::default();
        let mut model = Model::default();

        let mut cmd = app.update(Event::Get, &mut model);
        let mut request = cmd.expect_one_effect().expect_platform();

        request
            .resolve(PlatformResponse("platform".to_string()))
            .unwrap();

        let set_event = cmd.events().next().unwrap();
        assert_eq!(
            set_event,
            Event::Set(PlatformResponse("platform".to_string()))
        );

        let mut cmd = app.update(set_event, &mut model);
        cmd.expect_one_effect().expect_render();

        assert_eq!(model.platform, "platform");
        assert_eq!(app.view(&model).platform, "Hello platform");
    }
}
