use crux_core::render::Render;
use crux_platform::{Platform, PlatformResponse};
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

#[derive(crux_core::macros::Effect)]
pub struct Capabilities {
    pub(crate) platform: Platform<Event>,
    pub(crate) render: Render<Event>,
}

impl crux_core::App for App {
    type Event = Event;
    type Model = Model;
    type ViewModel = Model;
    type Capabilities = Capabilities;

    fn update(&self, msg: Event, model: &mut Model, caps: &Capabilities) {
        match msg {
            Event::Get => caps.platform.get(Event::Set),
            Event::Set(PlatformResponse(platform)) => {
                model.platform = platform;
                caps.render.render()
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
        let app = AppTester::<App, _>::default();
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
