use crux_core::{render::Render, App};
use crux_macros::Effect;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Event {
    Increment,
    Decrement,
    Reset,
}

#[derive(Default)]
pub struct Model {
    count: isize,
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
    type ViewModel = String;
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
        format!("Count is: {}", model.count)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crux_core::{render::RenderOperation, testing::AppTester};

    #[test]
    fn renders() {
        let app = AppTester::<Hello, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::Reset, &mut model);

        // Check update asked us to `Render`
        let actual_effect = &update.effects[0];
        let expected_effect = &Effect::Render(RenderOperation);
        assert_eq!(actual_effect, expected_effect);
    }

    #[test]
    fn shows_initial_count() {
        let app = AppTester::<Hello, _>::default();
        let mut model = Model::default();

        let actual_view = app.view(&mut model);
        let expected_view = "Count is: 0";
        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn increments_count() {
        let app = AppTester::<Hello, _>::default();
        let mut model = Model::default();

        app.update(Event::Increment, &mut model);

        let actual_view = app.view(&mut model);
        let expected_view = "Count is: 1";
        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn decrements_count() {
        let app = AppTester::<Hello, _>::default();
        let mut model = Model::default();

        app.update(Event::Decrement, &mut model);

        let actual_view = app.view(&mut model);
        let expected_view = "Count is: -1";
        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn resets_count() {
        let app = AppTester::<Hello, _>::default();
        let mut model = Model::default();

        app.update(Event::Increment, &mut model);
        app.update(Event::Reset, &mut model);

        let actual_view = app.view(&mut model);
        let expected_view = "Count is: 0";
        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn counts_up_and_down() {
        let app = AppTester::<Hello, _>::default();
        let mut model = Model::default();

        app.update(Event::Increment, &mut model);
        app.update(Event::Reset, &mut model);
        app.update(Event::Decrement, &mut model);
        app.update(Event::Increment, &mut model);
        app.update(Event::Increment, &mut model);

        let actual_view = app.view(&mut model);
        let expected_view = "Count is: 1";
        assert_eq!(actual_view, expected_view);
    }
}
