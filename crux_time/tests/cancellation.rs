use std::time::Duration;

use crux_core::{App, Command, Request};
use crux_time::{
    command::{CompletedTimerHandle, Time, TimerError, TimerHandle},
    TimeRequest, TimerId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    Start,
    Tick(Result<CompletedTimerHandle, TimerError>),
    Cancel,
}

pub enum Effect {
    Time(Request<TimeRequest>),
}

#[derive(Serialize, Deserialize)]
pub enum EffectFfi {
    Time(TimeRequest),
}

impl crux_core::Effect for Effect {
    type Ffi = EffectFfi;
    fn serialize(self) -> (Self::Ffi, crux_core::bridge::ResolveSerialized) {
        match self {
            Effect::Time(request) => request.serialize(EffectFfi::Time),
        }
    }
}

impl From<Request<TimeRequest>> for Effect {
    fn from(value: Request<TimeRequest>) -> Self {
        Self::Time(value)
    }
}

#[derive(Default)]
struct Model {
    timer_handle: Option<TimerHandle>,
    ticks: u32,
}

#[derive(Default)]
struct Timer;

impl App for Timer {
    type Event = Event;
    type Model = Model;
    type ViewModel = ();
    type Capabilities = ();
    type Effect = Effect;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        _caps: &Self::Capabilities,
    ) -> Command<Self::Effect, Self::Event> {
        match event {
            Event::Start => {
                let (request, handle) = Time::notify_after(Duration::from_secs(1));
                model.timer_handle = Some(handle);
                request.then_send(Event::Tick)
            }
            Event::Tick(response) => {
                if response.is_ok() {
                    model.ticks += 1;
                }

                Command::done()
            }
            Event::Cancel => {
                if let Some(handle) = model.timer_handle.take() {
                    println!("Timer cancelled");
                    handle.clear();
                }
                Command::done()
            }
        }
    }

    fn view(&self, _model: &Self::Model) -> Self::ViewModel {}
}

#[test]
fn ticking_timer_can_be_cancelled() {
    let app = Timer::default();
    let mut model = Model::default();

    const TIMER_ID: TimerId = TimerId(1);

    // start the timer
    let mut cmd1 = app.update(Event::Start, &mut model, &());

    let effect = cmd1.effects().next().unwrap();

    let Effect::Time(request) = effect;
    assert_eq!(
        request.operation,
        TimeRequest::NotifyAfter {
            id: TIMER_ID,
            duration: crux_time::Duration::from_secs(1)
        }
    );

    // cancel the timer
    let mut cmd2 = app.update(Event::Cancel, &mut model, &());
    assert!(cmd2.effects().next().is_none());

    // the first command should resolve with a TimeRequest::Clear
    let effect = cmd1.effects().next();

    let Some(Effect::Time(request)) = effect else {
        panic!("should get an effect");
    };

    let cancel_id = match &request.operation {
        TimeRequest::Clear { id } => id.clone(),
        _ => panic!("expected a Clear"),
    };
    assert_eq!(cancel_id, TIMER_ID);
}
