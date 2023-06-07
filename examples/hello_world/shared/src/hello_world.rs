use crux_core::{render::Render, App};
use crux_macros::Effect;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Event {
    None,
}

#[derive(Default)]
pub struct Model;

#[derive(Serialize, Deserialize)]
pub struct ViewModel {
    data: String,
}

#[derive(Effect)]
#[effect(app = "Hello")]
pub struct Capabilities {
    render: Render<Event>,
}

#[derive(Default)]
pub struct Hello;

impl App for Hello {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Capabilities = Capabilities;

    fn update(&self, _event: Self::Event, _model: &mut Self::Model, caps: &Self::Capabilities) {
        caps.render.render();
    }

    fn view(&self, _model: &Self::Model) -> Self::ViewModel {
        ViewModel {
            data: "Hello World".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crux_core::{assert_effect, testing::AppTester};

    #[test]
    fn hello_says_hello_world() {
        let hello = AppTester::<Hello, _>::default();
        let mut model = Model::default();

        // Call 'update' and request effects
        let update = hello.update(Event::None, &mut model);

        // Check update asked us to `Render`
        assert_effect!(update, Effect::Render(_));

        // Make sure the view matches our expectations
        let actual_view = &hello.view(&model).data;
        let expected_view = "Hello World";
        assert_eq!(actual_view, expected_view);
    }
}
