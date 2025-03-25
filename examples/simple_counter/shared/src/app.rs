// ANCHOR: app
use super::capabilities::delay::Delay;
use super::capabilities::interval::Interval;
use crux_core::{
    render::{render, Render},
    App, Command,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Event {
    StartInterval,
    DelayReset,
    Increment,
    Decrement,
    Reset,
}

#[derive(Default)]
pub struct Model {
    count: isize,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ViewModel {
    pub count: String,
}

#[cfg_attr(feature = "typegen", derive(crux_core::macros::Export))]
#[derive(crux_core::macros::Effect)]
#[allow(unused)]
pub struct Capabilities {
    render: Render<Event>,
    #[effect(stream_handler = crate::capabilities::interval::interval)]
    interval: Interval<Event>,
    #[effect(request_handler = crate::capabilities::delay::delay)]
    delay: Delay<Event>,
}

#[derive(Default)]
pub struct Counter;

// ANCHOR: impl_app
impl App for Counter {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Capabilities = Capabilities;
    type Effect = Effect;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        caps: &Self::Capabilities,
    ) -> Command<Effect, Event> {
        match event {
            Event::Increment => model.count += 1,
            Event::Decrement => model.count -= 1,
            Event::DelayReset => {
                caps.delay.start(5000, Event::Reset);
                return Command::done();
            }
            Event::Reset => model.count = 0,
            // Increment every one second, 10 times
            Event::StartInterval => {
                caps.interval.start(1000, 10, |_| Event::Increment);
                return Command::done();
            }
        };

        render()
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        ViewModel {
            count: format!("Count is: {}", model.count),
        }
    }
}
// ANCHOR_END: impl_app

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
