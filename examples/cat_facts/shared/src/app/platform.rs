use crux_core::{
    render::{self, Render},
    Command,
};
use crux_platform::{Platform, PlatformResponse};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct App {}

#[derive(Default, Serialize, Deserialize)]
pub struct Model {
    pub platform: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Event {
    Get,
    Set(PlatformResponse),
}

#[derive(crux_core::macros::Effect)]
pub struct Capabilities {
    pub platform: Platform<Event>,
    pub render: Render<Event>,
}

impl crux_core::App for App {
    type Event = Event;
    type Model = Model;
    type ViewModel = Model;
    type Capabilities = Capabilities;
    type Effect = Effect;

    fn update(&self, msg: Event, model: &mut Model, caps: &Capabilities) -> Command<Effect, Event> {
        match msg {
            Event::Get => {
                caps.platform.get(Event::Set);
                Command::done()
            }
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
    use crux_core::testing::AppTester;
    use crux_platform::PlatformResponse;

    use super::*;

    #[test]
    fn get_platform() {
        let app = AppTester::<App>::default();
        let mut model = Model::default();

        let request = &mut app
            .update(Event::Get, &mut model)
            .expect_one_effect()
            .expect_platform();

        let response = PlatformResponse("platform".to_string());
        app.resolve_to_event_then_update(request, response, &mut model)
            .expect_one_effect()
            .expect_render();

        assert_eq!(model.platform, "platform");
        assert_eq!(app.view(&model).platform, "Hello platform");
    }
}
