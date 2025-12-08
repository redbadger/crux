// ANCHOR: app
use crux_core::{
    App, Command,
    macros::effect,
    render::{RenderOperation, render},
};
use facet::Facet;
use serde::{Deserialize, Serialize};
use simple_moving_average::{NoSumSMA, SMA as _};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct DataPoint {
    pub id: u64,
    pub value: f64,
    pub label: String,
    pub metadata: Option<String>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub enum Event {
    Tick(Vec<DataPoint>),
    NewPeriod,
    Reset,
}

const SMA_WINDOW_SIZE: usize = 10;

#[derive(Debug, Default)]
pub struct Model {
    log: Vec<usize>,
    count: usize,
    last_payload: Vec<DataPoint>,
    max: usize,
    average: usize,
    sma: Option<NoSumSMA<usize, usize, SMA_WINDOW_SIZE>>,
    moving_average: usize,
}

impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        self.log == other.log
            && self.count == other.count
            && self.last_payload == other.last_payload
    }
}

#[derive(Facet, Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct ViewModel {
    pub count: usize,
    pub last_payload: Vec<DataPoint>,
    pub log: Vec<usize>,
    pub max: usize,
    pub average: usize,
    pub moving_average: usize,
}

#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
}

#[derive(Default)]
pub struct BridgeEcho;

// ANCHOR: impl_app
impl App for BridgeEcho {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Self::Event, model: &mut Self::Model) -> Command<Effect, Event> {
        match event {
            Event::Tick(payload) => {
                model.count += 1;
                model.last_payload = payload;
            }
            Event::NewPeriod => {
                model.log.push(model.count);
                model
                    .sma
                    .get_or_insert_with(NoSumSMA::<usize, usize, SMA_WINDOW_SIZE>::new)
                    .add_sample(model.count);
                model.max = *model.log.iter().max().unwrap_or(&0);
                model.average = if model.log.is_empty() {
                    0
                } else {
                    model.log.iter().sum::<usize>() / model.log.len()
                };
                model.moving_average = model.sma.unwrap().get_average();

                model.count = 0;
            }
            Event::Reset => {
                model.count = 0;
                model.log.clear();
            }
        }

        render()
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        ViewModel {
            count: model.count,
            last_payload: model.last_payload.clone(),
            log: model.log.clone(),
            max: model.max,
            average: model.average,
            moving_average: model.moving_average,
        }
    }
}
// ANCHOR_END: impl_app
// ANCHOR_END: app

// ANCHOR: test
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn shows_initial_count() {
        let app = BridgeEcho;
        let model = Model::default();

        let actual_view = app.view(&model);
        let expected_view = ViewModel::default();

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn increments_count() {
        let app = BridgeEcho;
        let mut model = Model::default();

        let _ = app.update(
            Event::Tick(vec![DataPoint::default(), DataPoint::default()]),
            &mut model,
        );
        let _ = app.update(Event::Tick(vec![DataPoint::default()]), &mut model);
        let _ = app.update(
            Event::Tick(vec![DataPoint::default(), DataPoint::default()]),
            &mut model,
        );

        let actual_view = app.view(&model);
        let expected_view = ViewModel {
            count: 3,
            last_payload: vec![DataPoint::default(), DataPoint::default()],
            ..Default::default()
        };

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn logs_previous_counts() {
        let app = BridgeEcho;
        let mut model = Model::default();

        let _ = app.update(Event::Tick(vec![DataPoint::default()]), &mut model);
        let _ = app.update(Event::Tick(vec![DataPoint::default()]), &mut model);
        let _ = app.update(Event::Tick(vec![DataPoint::default()]), &mut model);
        let _ = app.update(Event::NewPeriod, &mut model);
        let _ = app.update(Event::Tick(vec![DataPoint::default()]), &mut model);
        let _ = app.update(Event::Tick(vec![DataPoint::default()]), &mut model);
        let _ = app.update(Event::NewPeriod, &mut model);
        let _ = app.update(Event::Tick(vec![DataPoint::default()]), &mut model);

        let expected = Model {
            log: vec![3, 2],
            count: 1,
            last_payload: vec![DataPoint::default()],
            ..Default::default()
        };
        assert_eq!(model, expected);

        let expected = ViewModel {
            count: 1,
            log: vec![3, 2],
            last_payload: vec![DataPoint::default()],
            max: 3,
            average: 2,
            moving_average: 2,
        };
        let actual_view = app.view(&model);
        assert_eq!(actual_view, expected);
    }

    #[test]
    fn renders_on_tick() {
        let app = BridgeEcho;
        let mut model = Model::default();

        app.update(Event::Tick(vec![DataPoint::default()]), &mut model)
            .expect_one_effect()
            .expect_render();
    }

    #[test]
    fn renders_on_new_period() {
        let app = BridgeEcho;
        let mut model = Model::default();

        app.update(Event::NewPeriod, &mut model)
            .expect_one_effect()
            .expect_render();
    }
}
// ANCHOR_END: test
