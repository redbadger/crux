//! `spike(no_std)`: the `no_std` subset of `clock::Time`.
//!
//! Run with `cargo test -p crux_time --no-default-features --test nostd_clock`.
//! The library is built without `std` (the test harness itself still has std), so
//! only `notify_after`, the timer handles and the operation types are available.
#![cfg(not(feature = "std"))]

use core::time::Duration as CoreDuration;

use crux_core::{App, Command, Core, macros::effect};
use crux_time::{Duration, TimerHandle, TimerOutcome, clock::Time, operation};

#[effect]
pub enum Effect {
    TimeNotifyAfter(operation::NotifyAfter),
    TimeClear(operation::ClearTimer),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    Start,
    Cancel,
    Completed(TimerOutcome),
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    #[default]
    Idle,
    Pending,
    Completed,
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
    type ViewModel = Status;
    type Effect = Effect;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
    ) -> Command<Self::Effect, Self::Event> {
        match event {
            Event::Start => {
                let (request, handle) = Time::notify_after(CoreDuration::from_millis(500));
                model.handle = Some(handle);
                model.status = Status::Pending;
                request.then_send(Event::Completed)
            }
            Event::Cancel => {
                if let Some(handle) = model.handle.take() {
                    handle.clear();
                }
                Command::done()
            }
            Event::Completed(TimerOutcome::Completed(_)) => {
                model.status = Status::Completed;
                Command::done()
            }
            Event::Completed(TimerOutcome::Cleared) => {
                model.status = Status::Cleared;
                Command::done()
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        model.status
    }
}

#[test]
fn notify_after_completes_through_a_command() {
    let app = Timer;
    let mut model = Model::default();

    let mut cmd = app.update(Event::Start, &mut model);
    let mut request = cmd.expect_one_effect().expect_time_notify_after();
    assert_eq!(request.operation.duration, Duration::from_millis(500));

    let id = request.operation.id;
    request.resolve(id).expect("effect should resolve");

    let event = cmd.expect_one_event();
    let Event::Completed(TimerOutcome::Completed(completed)) = &event else {
        panic!("expected a completed timer, got {event:?}");
    };
    assert_eq!(
        completed,
        model.handle.as_ref().expect("the handle is stored")
    );

    app.update(event, &mut model).expect_done();
    assert_eq!(model.status, Status::Completed);
}

#[test]
fn clearing_a_timer_asks_the_shell() {
    let app = Timer;
    let mut model = Model::default();

    let mut cmd = app.update(Event::Start, &mut model);
    let id = cmd
        .expect_one_effect()
        .expect_time_notify_after()
        .operation
        .id;

    app.update(Event::Cancel, &mut model)
        .expect_no_effect_or_events();

    let mut clear = cmd.expect_one_effect().expect_time_clear();
    assert_eq!(clear.operation, operation::ClearTimer { id });
    cmd.expect_no_events();
    clear.resolve(id).expect("effect should resolve");

    let event = cmd.expect_one_event();
    assert_eq!(event, Event::Completed(TimerOutcome::Cleared));
    app.update(event, &mut model).expect_done();
    assert_eq!(model.status, Status::Cleared);
}

#[test]
fn notify_after_round_trips_through_a_core() {
    let core: Core<Timer> = Core::new();

    let mut effects = core.process_event(Event::Start);
    assert_eq!(effects.len(), 1);
    let Effect::TimeNotifyAfter(mut request) = effects.remove(0) else {
        panic!("expected a NotifyAfter");
    };
    assert_eq!(core.view(), Status::Pending);

    let id = request.operation.id;
    let effects = core
        .resolve(&mut request, id)
        .expect("effect should resolve");
    assert!(effects.is_empty());
    assert_eq!(core.view(), Status::Completed);
}
