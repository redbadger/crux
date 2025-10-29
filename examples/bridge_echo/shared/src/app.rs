// ANCHOR: app
use crux_core::{
    macros::effect,
    render::{render, RenderOperation},
    Command,
};
use facet::Facet;
use serde::{Deserialize, Serialize};

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

#[derive(Default, Debug, PartialEq)]
pub struct Model {
    log: Vec<usize>,
    count: usize,
    last_payload: Vec<DataPoint>,
}

#[derive(Facet, Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ViewModel {
    pub count: usize,
    pub log: Vec<usize>,
    pub last_payload: Vec<DataPoint>,
}

#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
}

#[derive(Default)]
pub struct App;

// ANCHOR: impl_app
impl crux_core::App for App {
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
        match event {
            Event::Tick(payload) => {
                model.count += 1;
                model.last_payload = payload;
            }
            Event::NewPeriod => {
                model.log.push(model.count);
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
            log: model.log.clone(),
            last_payload: model.last_payload.clone(),
        }
    }
}
// ANCHOR_END: impl_app
// ANCHOR_END: app

// ANCHOR: test
#[cfg(test)]
mod test {
    use crux_core::App as _;

    use super::*;

    #[test]
    fn shows_initial_count() {
        let app = App;
        let model = Model::default();

        let actual_view = app.view(&model);
        let expected_view = ViewModel {
            count: 0,
            log: vec![],
            last_payload: vec![],
        };

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn increments_count() {
        let app = App;
        let mut model = Model::default();

        let _ = app.update(
            Event::Tick(vec![DataPoint::default(), DataPoint::default()]),
            &mut model,
            &(),
        );
        let _ = app.update(Event::Tick(vec![DataPoint::default()]), &mut model, &());
        let _ = app.update(
            Event::Tick(vec![DataPoint::default(), DataPoint::default()]),
            &mut model,
            &(),
        );

        let actual_view = app.view(&model);
        let expected_view = ViewModel {
            count: 3,
            log: vec![],
            last_payload: vec![DataPoint::default(), DataPoint::default()],
        };

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn logs_previous_counts() {
        let app = App;
        let mut model = Model::default();

        let _ = app.update(Event::Tick(vec![DataPoint::default()]), &mut model, &());
        let _ = app.update(Event::Tick(vec![DataPoint::default()]), &mut model, &());
        let _ = app.update(Event::Tick(vec![DataPoint::default()]), &mut model, &());
        let _ = app.update(Event::NewPeriod, &mut model, &());
        let _ = app.update(Event::Tick(vec![DataPoint::default()]), &mut model, &());
        let _ = app.update(Event::Tick(vec![DataPoint::default()]), &mut model, &());
        let _ = app.update(Event::NewPeriod, &mut model, &());
        let _ = app.update(Event::Tick(vec![DataPoint::default()]), &mut model, &());

        let expected = Model {
            log: vec![3, 2],
            count: 1,
            last_payload: vec![DataPoint::default()],
        };
        assert_eq!(model, expected);
    }

    #[test]
    fn renders_on_tick() {
        let app = App;
        let mut model = Model::default();

        app.update(Event::Tick(vec![DataPoint::default()]), &mut model, &())
            .expect_one_effect()
            .expect_render();
    }

    #[test]
    fn renders_on_new_period() {
        let app = App;
        let mut model = Model::default();

        app.update(Event::NewPeriod, &mut model, &())
            .expect_one_effect()
            .expect_render();
    }
}
// ANCHOR_END: test
