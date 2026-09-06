//! The `Clock` API, driven through the per-operation types.

use std::time::{Duration as StdDuration, SystemTime};

use crux_core::{App, Command, macros::effect};
use crux_time::{Clock, Duration, Instant, TimerHandle, TimerId, TimerOutcome, operation};

#[effect]
pub enum Effect {
    Now(operation::Now),
    NotifyAt(operation::NotifyAt),
    NotifyAfter(operation::NotifyAfter),
    Clear(operation::Clear),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    // from the shell
    GetNow,
    Start,
    StartAt(SystemTime),
    Cancel,

    // from the core
    Now(SystemTime),
    Completed(TimerOutcome),
}

#[derive(Default, Debug, PartialEq, Eq)]
enum Status {
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
    now: Option<SystemTime>,
}

#[derive(Default)]
struct Timer;

impl App for Timer {
    type Event = Event;
    type Model = Model;
    type ViewModel = ();
    type Effect = Effect;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
    ) -> Command<Self::Effect, Self::Event> {
        match event {
            Event::GetNow => Clock::now().then_send(Event::Now),
            Event::Now(now) => {
                model.now = Some(now);
                Command::done()
            }
            Event::Start => {
                let (request, handle) = Clock::notify_after(StdDuration::from_secs(1));
                model.handle = Some(handle);
                model.status = Status::Pending;
                request.then_send(Event::Completed)
            }
            Event::StartAt(instant) => {
                let (request, handle) = Clock::notify_at(instant);
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
fn now_is_answered_with_an_instant() {
    let app = Timer;
    let mut model = Model::default();

    let mut cmd = app.update(Event::GetNow, &mut model);

    cmd.expect_no_events();
    let mut request = cmd.expect_one_effect().expect_now();
    assert_eq!(request.operation, operation::Now);

    request
        .resolve(Instant::new(1, 2))
        .expect("effect should resolve");

    let event = cmd.expect_one_event();
    app.update(event, &mut model).expect_no_effect_or_events();

    assert_eq!(
        model.now,
        Some(SystemTime::UNIX_EPOCH + StdDuration::new(1, 2))
    );
}

#[test]
fn notify_after_is_answered_with_its_timer_id() {
    let app = Timer;
    let mut model = Model::default();

    let mut cmd = app.update(Event::Start, &mut model);

    cmd.expect_no_events();
    let mut request = cmd.expect_one_effect().expect_notify_after();

    assert_eq!(request.operation.duration, Duration::from_secs(1));
    let timer_id = request.operation.id;

    request.resolve(timer_id).expect("effect should resolve");

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
fn notify_at_is_answered_with_its_timer_id() {
    let app = Timer;
    let mut model = Model::default();

    let at = SystemTime::UNIX_EPOCH + StdDuration::new(1, 2);
    let mut cmd = app.update(Event::StartAt(at), &mut model);

    cmd.expect_no_events();
    let mut request = cmd.expect_one_effect().expect_notify_at();

    assert_eq!(request.operation.instant, Instant::new(1, 2));
    let timer_id = request.operation.id;

    request.resolve(timer_id).expect("effect should resolve");

    let event = cmd.expect_one_event();
    app.update(event, &mut model).expect_done();

    assert_eq!(model.status, Status::Completed);
}

#[test]
fn clearing_a_timer_notifies_the_shell_and_does_not_wait_for_an_answer() {
    let app = Timer;
    let mut model = Model::default();

    // start the timer...
    let mut cmd = app.update(Event::Start, &mut model);

    // ...the shell is asked to start it
    let notify = cmd.expect_one_effect().expect_notify_after();
    let timer_id = notify.operation.id;
    assert_eq!(model.status, Status::Pending);

    // cancel the timer...
    app.update(Event::Cancel, &mut model)
        .expect_no_effect_or_events();
    assert_eq!(model.status, Status::Pending);

    // ...the original command tells the shell to clear it, as a notification
    let clear = cmd.expect_one_effect().expect_clear();
    assert_eq!(clear.operation, operation::Clear { id: timer_id });

    // ...and resolves straight away, without the shell answering
    let event = cmd.expect_one_event();
    assert_eq!(event, Event::Completed(TimerOutcome::Cleared));

    app.update(event, &mut model).expect_done();
    assert_eq!(model.status, Status::Cleared);
}

#[test]
fn dropping_a_timer_handle_does_not_clear_the_request() {
    let app = Timer;
    let mut model = Model::default();

    let mut cmd = app.update(Event::Start, &mut model);
    drop(model.handle.take());

    let mut request = cmd.expect_one_effect().expect_notify_after();
    let timer_id = request.operation.id;

    request.resolve(timer_id).expect("effect should resolve");

    let event = cmd.expect_one_event();
    assert!(matches!(
        event,
        Event::Completed(TimerOutcome::Completed(_))
    ));
}

#[test]
fn operations_serialize_to_json() {
    assert_eq!(serde_json::to_string(&operation::Now).unwrap(), "null");

    let notify_at = operation::NotifyAt {
        id: TimerId(1),
        instant: Instant::new(1, 2),
    };
    let serialized = serde_json::to_string(&notify_at).unwrap();
    assert_eq!(&serialized, r#"{"id":1,"instant":{"seconds":1,"nanos":2}}"#);
    assert_eq!(
        serde_json::from_str::<operation::NotifyAt>(&serialized).unwrap(),
        notify_at
    );

    let notify_after = operation::NotifyAfter {
        id: TimerId(2),
        duration: Duration::from_secs(1),
    };
    let serialized = serde_json::to_string(&notify_after).unwrap();
    assert_eq!(&serialized, r#"{"id":2,"duration":{"nanos":1000000000}}"#);
    assert_eq!(
        serde_json::from_str::<operation::NotifyAfter>(&serialized).unwrap(),
        notify_after
    );

    let clear = operation::Clear { id: TimerId(3) };
    let serialized = serde_json::to_string(&clear).unwrap();
    assert_eq!(&serialized, r#"{"id":3}"#);
    assert_eq!(
        serde_json::from_str::<operation::Clear>(&serialized).unwrap(),
        clear
    );
}

#[test]
fn outputs_serialize_to_json() {
    let instant = Instant::new(1, 2);
    assert_eq!(
        serde_json::to_string(&instant).unwrap(),
        r#"{"seconds":1,"nanos":2}"#
    );

    assert_eq!(serde_json::to_string(&TimerId(4)).unwrap(), "4");
}
