use crux_core::render::Render;
use crux_macros::Effect;
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

#[derive(Effect)]
pub struct Capabilities {
    pub platform: Platform<Event>,
    pub render: Render<Event>,
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
    use assert_let_bind::assert_let;
    use crux_core::testing::AppTester;
    use crux_platform::PlatformResponse;

    use super::*;

    #[test]
    fn get_platform() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let mut update = app.update(Event::Get, &mut model);

        assert_let!(Effect::Platform(request), &mut update.effects[0]);

        let response = PlatformResponse("platform".to_string());
        let update = app
            .resolve(request, response)
            .expect("should resolve successfully");
        for event in update.events {
            app.update(event, &mut model);
        }

        assert_eq!(model.platform, "platform");
        assert_eq!(app.view(&mut model).platform, "Hello platform");
    }
}
