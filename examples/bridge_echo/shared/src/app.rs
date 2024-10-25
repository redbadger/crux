// ANCHOR: app
use crux_core::render::Render;
use crux_core::Command;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Event {
    Tick,
    NewPeriod,
}

#[derive(Default, Debug, PartialEq)]
pub struct Model {
    log: Vec<usize>,
    count: usize,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ViewModel {
    pub count: usize,
}

#[cfg_attr(feature = "typegen", derive(crux_core::macros::Export))]
#[derive(crux_core::macros::Effect)]
pub struct Capabilities {
    render: Render,
}

#[derive(Default)]
pub struct App;

// ANCHOR: impl_app
impl crux_core::App for App {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Capabilities = Capabilities;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        caps: &Self::Capabilities,
    ) -> Command<Event> {
        match event {
            Event::Tick => model.count += 1,
            Event::NewPeriod => {
                model.log.push(model.count);
                model.count = 0;
            }
        };

        caps.render.render()
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        ViewModel { count: model.count }
    }
}
// ANCHOR_END: impl_app
// ANCHOR_END: app

// ANCHOR: test
#[cfg(test)]
mod test {
    use super::*;
    use crux_core::testing::AppTester;

    #[test]
    fn shows_initial_count() {
        let app = AppTester::<App, _>::default();
        let model = Model::default();

        let actual_view = app.view(&model);
        let expected_view = ViewModel { count: 0 };

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn increments_count() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let _ = app.update(Event::Tick, &mut model);
        let _ = app.update(Event::Tick, &mut model);
        let _ = app.update(Event::Tick, &mut model);

        let actual_view = app.view(&model);
        let expected_view = ViewModel { count: 3 };

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn logs_previous_counts() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let _ = app.update(Event::Tick, &mut model);
        let _ = app.update(Event::Tick, &mut model);
        let _ = app.update(Event::Tick, &mut model);
        let _ = app.update(Event::NewPeriod, &mut model);
        let _ = app.update(Event::Tick, &mut model);
        let _ = app.update(Event::Tick, &mut model);
        let _ = app.update(Event::NewPeriod, &mut model);
        let _ = app.update(Event::Tick, &mut model);

        let expected = Model {
            log: vec![3, 2],
            count: 1,
        };
        assert_eq!(model, expected);
    }

    #[test]
    fn renders_on_tick() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        app.update(Event::Tick, &mut model)
            .expect_one_effect()
            .expect_render();
    }

    #[test]
    fn renders_on_new_period() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        app.update(Event::NewPeriod, &mut model)
            .expect_one_effect()
            .expect_render();
    }
}
// ANCHOR_END: test
