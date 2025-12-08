// ANCHOR: app
use crux_core::{
    App, Command,
    macros::effect,
    render::{RenderOperation, render},
};
use facet::Facet;
use serde::{Deserialize, Serialize};

#[effect(facet_typegen)]
pub enum Effect {
    Render(RenderOperation),
}

#[derive(Facet, Serialize, Deserialize)]
#[repr(C)]
pub enum Event {
    None,
}

#[derive(Default)]
pub struct Model;

#[derive(Facet, Serialize, Deserialize)]
pub struct ViewModel {
    pub data: String,
}

#[derive(Default)]
pub struct HelloWorld;

impl App for HelloWorld {
    type Effect = Effect;
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;

    fn update(&self, _event: Self::Event, _model: &mut Self::Model) -> Command<Effect, Event> {
        render()
    }

    fn view(&self, _model: &Self::Model) -> Self::ViewModel {
        ViewModel {
            data: "Hello World".to_string(),
        }
    }
}
// ANCHOR_END: app

// ANCHOR: test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_says_hello_world() {
        let app = HelloWorld;
        let mut model = Model;

        // Call 'update' and request effects
        let mut cmd = app.update(Event::None, &mut model);

        // Check update asked us to `Render`
        cmd.expect_one_effect().expect_render();

        // Make sure the view matches our expectations
        let view = app.view(&model);
        let actual = &view.data;
        let expected = "Hello World";
        assert_eq!(actual, expected);
    }
}
// ANCHOR_END: test
