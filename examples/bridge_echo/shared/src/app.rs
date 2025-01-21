// ANCHOR: app
use crux_core::{
    render::{render, Render},
    Command,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Event {
    Tick,
    NewPeriod,
    Reset,
}

#[derive(Default, Debug, PartialEq)]
pub struct Model {
    log: Vec<usize>,
    count: usize,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ViewModel {
    pub count: usize,
    pub log: Vec<usize>,
}

#[cfg_attr(feature = "typegen", derive(crux_core::macros::Export))]
#[derive(crux_core::macros::Effect)]
#[allow(unused)]
pub struct Capabilities {
    render: Render<Event>,
}

#[derive(Default)]
pub struct App;

// ANCHOR: impl_app
impl crux_core::App for App {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Capabilities = Capabilities;
    type Effect = Effect;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        _caps: &Self::Capabilities,
    ) -> Command<Effect, Event> {
        match event {
            Event::Tick => model.count += 1,
            Event::NewPeriod => {
                model.log.push(model.count);
                model.count = 0;
            }
            Event::Reset => {
                model.count = 0;
                model.log.clear();
            }
        };

        render()
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        ViewModel {
            count: model.count,
            log: model.log.clone(),
        }
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
        let app = AppTester::<App>::default();
        let model = Model::default();

        let actual_view = app.view(&model);
        let expected_view = ViewModel {
            count: 0,
            log: vec![],
        };

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn increments_count() {
        let app = AppTester::<App>::default();
        let mut model = Model::default();

        let _ = app.update(Event::Tick, &mut model);
        let _ = app.update(Event::Tick, &mut model);
        let _ = app.update(Event::Tick, &mut model);

        let actual_view = app.view(&model);
        let expected_view = ViewModel {
            count: 3,
            log: vec![],
        };

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn logs_previous_counts() {
        let app = AppTester::<App>::default();
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
        let app = AppTester::<App>::default();
        let mut model = Model::default();

        app.update(Event::Tick, &mut model)
            .expect_one_effect()
            .expect_render();
    }

    #[test]
    fn renders_on_new_period() {
        let app = AppTester::<App>::default();
        let mut model = Model::default();

        app.update(Event::NewPeriod, &mut model)
            .expect_one_effect()
            .expect_render();
    }
}
// ANCHOR_END: test
