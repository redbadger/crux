// ANCHOR: app
use crux_core::{
    macros::effect2,
    render::{render, RenderOperation},
    App, Command,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Event {
    Increment,
    Decrement,
    Reset,
}

effect2! {
    pub enum Effect {
        Render(RenderOperation),
    }
}

#[derive(Default)]
pub struct Model {
    count: isize,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ViewModel {
    pub count: String,
}

#[derive(Default)]
pub struct Counter;

// ANCHOR: impl_app
impl App for Counter {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Capabilities = ();
    type Effect = Effect;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        _caps: &Self::Capabilities,
    ) -> Command<Effect, Event> {
        // we no longer use the capabilities directly, but they are passed in
        // until the migration to managed effects with `Command` is complete
        // (at which point the capabilities will be removed from the `update`
        // signature). Until then we delegate to our own `update` method so that
        // we can test the app without needing to use AppTester.
        self.update(event, model)
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        ViewModel {
            count: format!("Count is: {}", model.count),
        }
    }
}
// ANCHOR_END: impl_app

impl Counter {
    // note: this function can be moved into the `App` trait implementation, above,
    // once the `App` trait has been updated (as the final part of the migration
    // to managed effects with `Command`).
    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::Increment => model.count += 1,
            Event::Decrement => model.count -= 1,
            Event::Reset => model.count = 0,
        };

        render()
    }
}
// ANCHOR_END: app

// ANCHOR: test
#[cfg(test)]
mod test {
    use super::*;
    use crux_core::assert_effect;

    #[test]
    fn renders() {
        let app = Counter::default();
        let mut model = Model::default();

        let mut cmd = app.update(Event::Reset, &mut model);

        // Check update asked us to `Render`
        assert_effect!(cmd, Effect::Render(_));
    }

    #[test]
    fn shows_initial_count() {
        let app = Counter::default();
        let model = Model::default();

        let actual_view = app.view(&model).count;
        let expected_view = "Count is: 0";
        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn increments_count() {
        let app = Counter::default();
        let mut model = Model::default();

        let mut cmd = app.update(Event::Increment, &mut model);

        let actual_view = app.view(&model).count;
        let expected_view = "Count is: 1";
        assert_eq!(actual_view, expected_view);

        // Check update asked us to `Render`
        assert_effect!(cmd, Effect::Render(_));
    }

    #[test]
    fn decrements_count() {
        let app = Counter::default();
        let mut model = Model::default();

        let mut cmd = app.update(Event::Decrement, &mut model);

        let actual_view = app.view(&model).count;
        let expected_view = "Count is: -1";
        assert_eq!(actual_view, expected_view);

        // Check update asked us to `Render`
        assert_effect!(cmd, Effect::Render(_));
    }

    #[test]
    fn resets_count() {
        let app = Counter::default();
        let mut model = Model::default();

        let _ = app.update(Event::Increment, &mut model);
        let _ = app.update(Event::Reset, &mut model);

        let actual_view = app.view(&model).count;
        let expected_view = "Count is: 0";
        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn counts_up_and_down() {
        let app = Counter::default();
        let mut model = Model::default();

        let _ = app.update(Event::Increment, &mut model);
        let _ = app.update(Event::Reset, &mut model);
        let _ = app.update(Event::Decrement, &mut model);
        let _ = app.update(Event::Increment, &mut model);
        let _ = app.update(Event::Increment, &mut model);

        let actual_view = app.view(&model).count;
        let expected_view = "Count is: 1";
        assert_eq!(actual_view, expected_view);
    }
}
// ANCHOR_END: test
