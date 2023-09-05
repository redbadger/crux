use std::time::Duration;

// ANCHOR: app
use crux_core::render::Render;
use crux_macros::Effect;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Event {
    Start(usize),
    Stop,
    Tick,
    NewPeriod,
}

const EMA_ALPHA: f64 = 0.2; // TODO tune!

#[derive(Default, Debug, PartialEq)]
pub struct Model {
    sample_period: Option<Duration>,
    log: Vec<usize>,
    count: usize,
    rate: f64, // per second
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ViewModel {
    pub count: usize,
    pub rate: f64,
    pub log: Vec<usize>,
    pub running: bool,
}

#[cfg_attr(feature = "typegen", derive(crux_macros::Export))]
#[derive(Effect)]
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

    fn update(&self, event: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) {
        match event {
            Event::Start(msecs) => {
                if model.sample_period.is_some() {
                    return;
                }

                model.count = 0;
                model.rate = 0.0;
                model.log = vec![];
                model.sample_period = Some(Duration::from_millis(msecs as u64));
            }
            Event::Stop => {
                if model.sample_period.is_none() {
                    return;
                }

                model.sample_period = None;
            }
            Event::Tick => {
                if model.sample_period.is_none() {
                    return;
                }

                model.count += 1
            }
            Event::NewPeriod => {
                let Some(period_duration) = model.sample_period else {
                    return;
                };

                // Normalise count to 'per second' scale
                let count_per_second =
                    model.count as f64 * (1000.0 / period_duration.as_millis() as f64);

                model.log.push(count_per_second as usize);

                // Filter with an exponential moving average
                model.rate = if model.rate > 0.0 {
                    model.rate * (1.0 - EMA_ALPHA) + EMA_ALPHA * count_per_second
                } else {
                    count_per_second
                };

                model.count = 0;
            }
        };

        caps.render.render();
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        ViewModel {
            count: model.count,
            rate: model.rate,
            log: model.log.clone(),
            running: model.sample_period.is_some(),
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
    fn start_resets_everything() {
        let app = AppTester::<App, _>::default();
        let mut model = Model {
            sample_period: None,
            log: vec![20, 23, 42],
            count: 57,
            rate: 45.0,
        };

        app.update(Event::Start(1000), &mut model);

        let expected = Model {
            sample_period: Some(Duration::from_millis(1000)),
            log: vec![],
            count: 0,
            rate: 0.0,
        };

        assert_eq!(model, expected);
    }

    #[test]
    fn start_does_nothing_when_already_running() {
        let app = AppTester::<App, _>::default();
        let mut model = Model {
            sample_period: Some(Duration::from_millis(300)),
            log: vec![20, 23, 42],
            count: 57,
            rate: 45.0,
        };

        app.update(Event::Start(1000), &mut model);

        let expected = Model {
            sample_period: Some(Duration::from_millis(300)),
            log: vec![20, 23, 42],
            count: 57,
            rate: 45.0,
        };

        assert_eq!(model, expected);
    }

    #[test]
    fn stop_resets_sample_period() {
        let app = AppTester::<App, _>::default();
        let mut model = Model {
            sample_period: Some(Duration::from_millis(300)),
            log: vec![20, 23, 42],
            count: 57,
            rate: 45.0,
        };

        app.update(Event::Stop, &mut model);

        let expected = Model {
            sample_period: None,
            log: vec![20, 23, 42],
            count: 57,
            rate: 45.0,
        };

        assert_eq!(model, expected);
    }

    #[test]
    fn stop_does_nothing_when_not_running() {
        let app = AppTester::<App, _>::default();
        let mut model = Model {
            sample_period: None,
            log: vec![20, 23, 42],
            count: 57,
            rate: 45.0,
        };

        app.update(Event::Stop, &mut model);

        let expected = Model {
            sample_period: None,
            log: vec![20, 23, 42],
            count: 57,
            rate: 45.0,
        };

        assert_eq!(model, expected);
    }

    #[test]
    fn shows_initial_count() {
        let app = AppTester::<App, _>::default();
        let model = Model::default();

        let actual_view = app.view(&model);
        let expected_view = ViewModel {
            count: 0,
            rate: 0.0,
            log: vec![],
            running: false,
        };

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn increments_count_when_running() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        app.update(Event::Start(500), &mut model);
        app.update(Event::Tick, &mut model);
        app.update(Event::Tick, &mut model);
        app.update(Event::Tick, &mut model);

        let actual_view = app.view(&model);
        let expected_view = ViewModel {
            count: 3,
            rate: 0.0,
            log: vec![],
            running: true,
        };

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn ignores_tick_when_not_running() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        app.update(Event::Tick, &mut model);
        app.update(Event::Tick, &mut model);
        app.update(Event::Tick, &mut model);

        let actual_view = app.view(&model);
        let expected_view = ViewModel {
            count: 0,
            rate: 0.0,
            log: vec![],
            running: false,
        };

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn logs_previous_counts() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        app.update(Event::Start(200), &mut model);
        app.update(Event::Tick, &mut model);
        app.update(Event::Tick, &mut model);
        app.update(Event::Tick, &mut model);
        app.update(Event::NewPeriod, &mut model);
        app.update(Event::Tick, &mut model);
        app.update(Event::Tick, &mut model);
        app.update(Event::NewPeriod, &mut model);
        app.update(Event::Tick, &mut model);

        let expected = Model {
            sample_period: Some(Duration::from_millis(200)),
            log: vec![15, 10],
            count: 1,
            rate: 15.0 * (1.0 - EMA_ALPHA) + 10.0 * EMA_ALPHA,
        };
        assert_eq!(model, expected);
    }

    #[test]
    fn renders_on_tick() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        app.update(Event::Start(500), &mut model);
        let update = app.update(Event::Tick, &mut model);

        assert_effect!(update, Effect::Render(_));
    }

    #[test]
    fn renders_on_new_period() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        app.update(Event::Start(500), &mut model);
        let update = app.update(Event::NewPeriod, &mut model);

        assert_effect!(update, Effect::Render(_));
    }
}
// ANCHOR_END: test
