use std::time::Duration;

use crux_core::{App, Command, Request};
use crux_time::{
    command::{CompletedTimerHandle, Time, TimerError, TimerHandle},
    TimeRequest, TimeResponse, TimerId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    // from shell
    Start,
    Cancel,

    // from core
    Stop(Result<CompletedTimerHandle, TimerError>),
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

#[derive(Default, Debug, PartialEq, Eq)]
enum Status {
    #[default]
    Stopped,
    Started,
    Cancelled,
}

#[derive(Default)]
struct Model {
    handle: Option<TimerHandle>,
    status: Status,
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
                model.handle = Some(handle);
                model.status = Status::Started;
                request.then_send(Event::Stop)
            }
            Event::Stop(response) => {
                if response.is_ok() {
                    model.status = Status::Stopped;
                }

                Command::done()
            }
            Event::Cancel => {
                if let Some(handle) = model.handle.take() {
                    println!("Timer cancelled");
                    handle.clear();
                    model.status = Status::Cancelled;
                }
                Command::done()
            }
        }
    }

    fn view(&self, _model: &Self::Model) -> Self::ViewModel {}
}

#[test]
fn cancellation_of_a_started_timer() {
    let app = Timer;
    let mut model = Model::default();

    const TIMER_ID: TimerId = TimerId(1);

    // start the timer...
    let mut cmd1 = app.update(Event::Start, &mut model, &());

    // ...no events
    assert!(cmd1.events().next().is_none());

    // ...but an effect to ask the shell to start the timer
    let Effect::Time(request) = cmd1.effects().next().unwrap();
    assert_eq!(
        request.operation,
        TimeRequest::NotifyAfter {
            id: TIMER_ID,
            duration: crux_time::Duration::from_secs(1)
        }
    );

    // ...and the model is updated
    assert_eq!(model.status, Status::Started);

    // cancel the timer...
    let mut cmd2 = app.update(Event::Cancel, &mut model, &());

    // ...no events or effects
    assert!(cmd2.events().next().is_none());
    assert!(cmd2.effects().next().is_none());
    // ...but the model is updated
    assert_eq!(model.status, Status::Cancelled);

    // ...however, the _first_ command should resolve with a TimeRequest::Clear
    // so that the shell can clean up
    let Effect::Time(mut request) = cmd1.effects().next().unwrap();
    let cancel_id = match &request.operation {
        TimeRequest::Clear { id } => *id,
        _ => panic!("expected a Clear"),
    };
    assert_eq!(cancel_id, TIMER_ID);

    // note, the shell does _not_ need to respond to say it has cleaned up,
    // but if it does, the core should ignore it, as it has already been cancelled...
    let response = TimeResponse::Cleared { id: TIMER_ID };
    request.resolve(response).unwrap();

    // ...no effects
    assert!(cmd1.effects().next().is_none());
    // ...but one event with an error to say it is already cleared,
    assert_eq!(
        cmd1.events().next().unwrap(),
        Event::Stop(Err(TimerError::Cleared))
    );
    // ...and the model is not updated
    assert_eq!(model.status, Status::Cancelled);
}
