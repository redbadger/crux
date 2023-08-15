// ANCHOR: app
use crux_core::{render::Render, App};
use crux_macros::Effect;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Event {
    Increment,
    Decrement,
    Reset,
}

#[derive(Default)]
pub struct Model {
    count: isize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ViewModel {
    pub count: String,
}

#[cfg_attr(feature = "typegen", derive(crux_macros::Export))]
#[derive(Effect)]
#[effect(app = "Counter")]
pub struct Capabilities {
    render: Render<Event>,
}

#[derive(Default)]
pub struct Counter;

// ANCHOR: impl_app
impl App for Counter {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Capabilities = Capabilities;

    fn update(&self, event: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) {
        match event {
            Event::Increment => model.count += 1,
            Event::Decrement => model.count -= 1,
            Event::Reset => model.count = 0,
        };

        caps.render.render();
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        ViewModel {
            count: format!("Count is: {}", model.count),
        }
    }
}
// ANCHOR_END: impl_app
// ANCHOR_END: app

// ANCHOR: test
#[cfg(test)]
mod test {
    use super::*;
    use crux_core::{assert_effect, testing::AppTester};

    #[test]
    fn renders() {
        let app = AppTester::<Counter, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::Reset, &mut model);

        // Check update asked us to `Render`
        assert_effect!(update, Effect::Render(_));
    }

    #[test]
    fn shows_initial_count() {
        let app = AppTester::<Counter, _>::default();
        let model = Model::default();

        let actual_view = app.view(&model).count;
        let expected_view = "Count is: 0";
        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn increments_count() {
        let app = AppTester::<Counter, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::Increment, &mut model);

        let actual_view = app.view(&model).count;
        let expected_view = "Count is: 1";
        assert_eq!(actual_view, expected_view);

        // Check update asked us to `Render`
        assert_effect!(update, Effect::Render(_));
    }

    #[test]
    fn decrements_count() {
        let app = AppTester::<Counter, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::Decrement, &mut model);

        let actual_view = app.view(&model).count;
        let expected_view = "Count is: -1";
        assert_eq!(actual_view, expected_view);

        // Check update asked us to `Render`
        assert_effect!(update, Effect::Render(_));
    }

    #[test]
    fn resets_count() {
        let app = AppTester::<Counter, _>::default();
        let mut model = Model::default();

        app.update(Event::Increment, &mut model);
        app.update(Event::Reset, &mut model);

        let actual_view = app.view(&model).count;
        let expected_view = "Count is: 0";
        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn counts_up_and_down() {
        let app = AppTester::<Counter, _>::default();
        let mut model = Model::default();

        app.update(Event::Increment, &mut model);
        app.update(Event::Reset, &mut model);
        app.update(Event::Decrement, &mut model);
        app.update(Event::Increment, &mut model);
        app.update(Event::Increment, &mut model);

        let actual_view = app.view(&model).count;
        let expected_view = "Count is: 1";
        assert_eq!(actual_view, expected_view);
    }
}
// ANCHOR_END: test
