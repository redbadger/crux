use crux_core::{
    macros::effect,
    render::{self, RenderOperation},
    Command,
};
use crux_platform::{command::Platform, PlatformRequest, PlatformResponse};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct App {}

#[derive(Default, Serialize, Deserialize)]
pub struct Model {
    pub(crate) platform: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Event {
    Get,
    Set(PlatformResponse),
}

#[effect]
pub enum Effect {
    Platform(PlatformRequest),
    Render(RenderOperation),
}

impl crux_core::App for App {
    type Event = Event;
    type Model = Model;
    type ViewModel = Model;
    type Capabilities = ();
    type Effect = Effect;

    fn update(&self, msg: Event, model: &mut Model, _caps: &()) -> Command<Effect, Event> {
        self.update(msg, model)
    }

    fn view(&self, model: &Model) -> Model {
        Model {
            platform: format!("Hello {platform}", platform = model.platform),
        }
    }
}

impl App {
    pub fn update(&self, msg: Event, model: &mut Model) -> Command<Effect, Event> {
        match msg {
            Event::Get => Platform::get().then_send(Event::Set),
            Event::Set(PlatformResponse(platform)) => {
                model.platform = platform;
                render::render()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crux_core::App as _;
    use crux_platform::PlatformResponse;

    use super::*;

    #[test]
    fn get_platform() {
        let app = App::default();
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
