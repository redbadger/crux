// ANCHOR: app

use crux_core::{
    render::{render, Render},
    App, Command,
};
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

#[derive(crux_core::macros::Effect)]
#[allow(unused)]
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
    type Effect = Effect;

    fn update(
        &self,
        _event: Self::Event,
        _model: &mut Self::Model,
        _caps: &Self::Capabilities,
    ) -> Command<Effect, Event> {
        // we no longer use the capabilities directly, but they are passed in
        // until the migration to managed effects with `Command` is complete
        // (at which point the capabilities will be removed from the `update`
        // signature). Until then we delegate to our own `update` method so that
        // we can test the app without needing to use AppTester.
        self.update(_event, _model)
    }

    fn view(&self, _model: &Self::Model) -> Self::ViewModel {
        ViewModel {
            data: "Hello World".to_string(),
        }
    }
}

impl Hello {
    // note: this function can be moved into the `App` trait implementation, above,
    // once the `App` trait has been updated (as the final part of the migration
    // to managed effects with `Command`).
    fn update(&self, _event: Event, _model: &mut Model) -> Command<Effect, Event> {
        render()
    }
}

// ANCHOR_END: app

// ANCHOR: test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_says_hello_world() {
        let hello = Hello::default();
        let mut model = Model;

        // Call 'update' and request effects
        let mut cmd = hello.update(Event::None, &mut model);

        // Check update asked us to `Render`
        cmd.expect_one_effect().expect_render();

        // Make sure the view matches our expectations
        let actual_view = &hello.view(&model).data;
        let expected_view = "Hello World";
        assert_eq!(actual_view, expected_view);
    }
}

// ANCHOR_END: test
