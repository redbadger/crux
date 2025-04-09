use std::time::Duration;

use crux_core::{macros::effect, App, Command};
use crux_time::{
    command::{Time, TimerHandle, TimerOutcome},
    TimeRequest, TimeResponse, TimerId,
};

#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    // from shell
    Start,
    Cancel,

    // from core
    Completed(TimerOutcome),
}

#[effect]
pub enum Effect {
    Time(TimeRequest),
}

#[derive(Default, Debug, PartialEq, Eq)]
enum Status {
    #[default]
    Completed,
    Pending,
    Cleared,
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
                model.status = Status::Pending;
                request.then_send(Event::Completed)
            }
            Event::Completed(TimerOutcome::Completed(_)) => {
                model.status = Status::Completed;
                Command::done()
            }
            Event::Completed(TimerOutcome::Cleared) => {
                model.status = Status::Cleared;
                Command::done()
            }
            Event::Cancel => {
                if let Some(handle) = model.handle.take() {
                    handle.clear();
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
    assert_eq!(model.status, Status::Pending);

    // cancel the timer...
    let mut cmd2 = app.update(Event::Cancel, &mut model, &());

    // ...no events or effects
    assert!(cmd2.events().next().is_none());
    assert!(cmd2.effects().next().is_none());
    // ...and the model is not yet updated (still at pending)
    assert_eq!(model.status, Status::Pending);

    // ...however, the _first_ command resolves with a TimeRequest::Clear
    // so that the shell can clean up
    let Effect::Time(mut request) = cmd1.effects().next().unwrap();
    let cancel_id = match &request.operation {
        TimeRequest::Clear { id } => *id,
        _ => panic!("expected a Clear"),
    };
    assert_eq!(cancel_id, TIMER_ID);

    // the shell then responds, to say it has cleaned up
    let response = TimeResponse::Cleared { id: TIMER_ID };
    request.resolve(response).unwrap();

    // ...no effects
    assert!(cmd1.effects().next().is_none());
    // ...and one event to signify that the timer has been cleared
    let event = cmd1.events().next().unwrap();
    assert_eq!(&event, &Event::Completed(TimerOutcome::Cleared));

    // now we send the event back into the app
    let _ = app.update(event, &mut model, &());

    // ...and the model is updated
    assert_eq!(model.status, Status::Cleared);
}
