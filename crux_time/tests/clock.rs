//! The `clock::Time` API, driven through the per-operation types.

use std::time::{Duration as StdDuration, SystemTime};

use crux_core::{App, Command, macros::effect};
use crux_time::{Duration, Instant, TimerHandle, TimerId, TimerOutcome, clock::Time, operation};
use futures::StreamExt;

#[effect]
pub enum Effect {
    Now(operation::Now),
    NotifyAt(operation::NotifyAt),
    NotifyAfter(operation::NotifyAfter),
    NotifyEvery(operation::NotifyEvery),
    Clear(operation::Clear),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    // from the shell
    GetNow,
    Start,
    StartAt(SystemTime),
    StartEvery,
    StartEveryOnce,
    Cancel,

    // from the core
    Now(SystemTime),
    Completed(TimerOutcome),
    Tick(TimerOutcome),
    FirstTick(Option<TimerOutcome>),
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
    ticks: usize,
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
            Event::GetNow => Time::now().then_send(Event::Now),
            Event::Now(now) => {
                model.now = Some(now);
                Command::done()
            }
            Event::Start => {
                let (request, handle) = Time::notify_after(StdDuration::from_secs(1));
                model.handle = Some(handle);
                model.status = Status::Pending;
                request.then_send(Event::Completed)
            }
            Event::StartAt(instant) => {
                let (request, handle) = Time::notify_at(instant);
                model.handle = Some(handle);
                model.status = Status::Pending;
                request.then_send(Event::Completed)
            }
            Event::StartEvery => {
                let (ticks, handle) = Time::notify_every(StdDuration::from_secs(1));
                model.handle = Some(handle);
                model.status = Status::Pending;
                ticks.then_send(Event::Tick)
            }
            Event::StartEveryOnce => {
                let (builder, handle) = Time::notify_every(StdDuration::from_secs(1));
                model.handle = Some(handle);
                model.status = Status::Pending;

                // take the first tick, then drop the stream
                Command::new(|ctx| async move {
                    let mut ticks = builder.into_stream(ctx.clone());
                    let first = ticks.next().await;
                    drop(ticks);

                    ctx.send_event(Event::FirstTick(first));
                })
            }
            Event::FirstTick(_) => Command::done(),
            Event::Tick(TimerOutcome::Completed(_)) => {
                model.ticks += 1;
                Command::done()
            }
            Event::Completed(TimerOutcome::Completed(_)) => {
                model.status = Status::Completed;
                Command::done()
            }
            Event::Tick(TimerOutcome::Cleared) | Event::Completed(TimerOutcome::Cleared) => {
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
fn clearing_a_timer_asks_the_shell_and_waits_for_its_answer() {
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

    // ...the original command asks the shell to clear it
    let mut clear = cmd.expect_one_effect().expect_clear();
    assert_eq!(clear.operation, operation::Clear { id: timer_id });

    // ...and nothing happens until the shell has answered
    cmd.expect_no_events();
    clear.resolve(timer_id).expect("effect should resolve");

    let event = cmd.expect_one_event();
    assert_eq!(event, Event::Completed(TimerOutcome::Cleared));

    app.update(event, &mut model).expect_done();
    assert_eq!(model.status, Status::Cleared);
}

#[test]
fn answering_a_cleared_timer_late_is_ignored() {
    let app = Timer;
    let mut model = Model::default();

    // start the timer, and hang on to the shell's side of the request
    let mut cmd = app.update(Event::Start, &mut model);
    let mut notify = cmd.expect_one_effect().expect_notify_after();
    let timer_id = notify.operation.id;

    // cancel it...
    app.update(Event::Cancel, &mut model)
        .expect_no_effect_or_events();
    let mut clear = cmd.expect_one_effect().expect_clear();
    assert_eq!(clear.operation, operation::Clear { id: timer_id });

    // ...and only then does the shell get round to answering the original
    // `NotifyAfter`. The core has stopped listening, so the answer is dropped:
    // resolving succeeds, and produces nothing.
    notify.resolve(timer_id).expect("effect should resolve");
    cmd.expect_no_effect_or_events();

    // the `Clear` still resolves as normal
    clear.resolve(timer_id).expect("effect should resolve");
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
fn notify_every_is_answered_with_its_timer_id() {
    let app = Timer;
    let mut model = Model::default();

    let mut cmd = app.update(Event::StartEvery, &mut model);

    cmd.expect_no_events();
    let mut request = cmd.expect_one_effect().expect_notify_every();

    assert_eq!(request.operation.period, Duration::from_secs(1));
    let timer_id = request.operation.id;

    request.resolve(timer_id).expect("effect should resolve");

    let event = cmd.expect_one_event();
    let Event::Tick(TimerOutcome::Completed(completed)) = &event else {
        panic!("expected a tick, got {event:?}");
    };
    assert_eq!(
        completed,
        model.handle.as_ref().expect("the handle is stored")
    );

    app.update(event, &mut model).expect_done();
    assert_eq!(model.ticks, 1);
    assert!(!cmd.is_done());
}

#[test]
fn notify_every_ticks_many_times_from_one_request() {
    let app = Timer;
    let mut model = Model::default();

    let mut cmd = app.update(Event::StartEvery, &mut model);
    let mut request = cmd.expect_one_effect().expect_notify_every();
    let timer_id = request.operation.id;

    for _ in 0..3 {
        request.resolve(timer_id).expect("effect should resolve");

        // each tick carries the same timer ID, and asks the shell for nothing
        let event = cmd.expect_one_event();
        let Event::Tick(TimerOutcome::Completed(completed)) = &event else {
            panic!("expected a tick, got {event:?}");
        };
        assert_eq!(
            completed,
            model.handle.as_ref().expect("the handle is stored")
        );
        cmd.expect_no_effect_or_events();

        app.update(event, &mut model).expect_done();
    }

    assert_eq!(model.ticks, 3);
    assert!(!cmd.is_done());
}

#[test]
fn clearing_a_repeating_timer_before_it_starts_asks_the_shell_nothing() {
    let app = Timer;
    let mut model = Model::default();

    // start the timer, and clear it before the command runs
    let mut cmd = app.update(Event::StartEvery, &mut model);
    app.update(Event::Cancel, &mut model)
        .expect_no_effect_or_events();

    // the shell gets no request, and the stream yields `Cleared` once
    cmd.expect_no_effects();
    let event = cmd.expect_one_event();
    assert_eq!(event, Event::Tick(TimerOutcome::Cleared));
    assert!(cmd.is_done());

    app.update(event, &mut model).expect_done();
    assert_eq!(model.status, Status::Cleared);
}

#[test]
fn clearing_a_repeating_timer_asks_the_shell_and_waits_for_its_answer() {
    let app = Timer;
    let mut model = Model::default();

    // start the timer, and let it tick once
    let mut cmd = app.update(Event::StartEvery, &mut model);
    let mut notify = cmd.expect_one_effect().expect_notify_every();
    let timer_id = notify.operation.id;

    notify.resolve(timer_id).expect("effect should resolve");
    let event = cmd.expect_one_event();
    app.update(event, &mut model).expect_done();
    assert_eq!(model.ticks, 1);

    // cancel the timer...
    app.update(Event::Cancel, &mut model)
        .expect_no_effect_or_events();

    // ...the command asks the shell to clear it
    let mut clear = cmd.expect_one_effect().expect_clear();
    assert_eq!(clear.operation, operation::Clear { id: timer_id });

    // ...and nothing happens until the shell has answered
    cmd.expect_no_events();
    assert!(!cmd.is_done());
    clear.resolve(timer_id).expect("effect should resolve");

    let event = cmd.expect_one_event();
    assert_eq!(event, Event::Tick(TimerOutcome::Cleared));
    assert!(cmd.is_done());

    app.update(event, &mut model).expect_done();
    assert_eq!(model.status, Status::Cleared);
    assert_eq!(model.ticks, 1);
}

#[test]
fn ticking_a_cleared_repeating_timer_late_is_ignored() {
    let app = Timer;
    let mut model = Model::default();

    // start the timer, and keep the shell's side of the stream
    let mut cmd = app.update(Event::StartEvery, &mut model);
    let mut notify = cmd.expect_one_effect().expect_notify_every();
    let timer_id = notify.operation.id;

    // cancel it...
    app.update(Event::Cancel, &mut model)
        .expect_no_effect_or_events();
    let mut clear = cmd.expect_one_effect().expect_clear();
    assert_eq!(clear.operation, operation::Clear { id: timer_id });

    // ...and only then does the shell send another tick. The core no longer
    // listens, so the resolve returns an error and produces nothing.
    notify
        .resolve(timer_id)
        .expect_err("the core no longer listens for ticks");
    cmd.expect_no_effect_or_events();

    // the `Clear` still resolves as normal
    clear.resolve(timer_id).expect("effect should resolve");
    let event = cmd.expect_one_event();
    assert_eq!(event, Event::Tick(TimerOutcome::Cleared));

    app.update(event, &mut model).expect_done();
    assert_eq!(model.status, Status::Cleared);
    assert_eq!(model.ticks, 0);
}

#[test]
fn dropping_a_repeating_timer_handle_before_it_starts_keeps_it_ticking() {
    let app = Timer;
    let mut model = Model::default();

    let mut cmd = app.update(Event::StartEvery, &mut model);
    drop(model.handle.take());

    let mut request = cmd.expect_one_effect().expect_notify_every();
    let timer_id = request.operation.id;

    for _ in 0..2 {
        request.resolve(timer_id).expect("effect should resolve");
        let event = cmd.expect_one_event();
        assert!(matches!(event, Event::Tick(TimerOutcome::Completed(_))));
        cmd.expect_no_effect_or_events();
    }
    assert!(!cmd.is_done());

    // the app stops the ticks by aborting the command
    cmd.abort_handle().abort();
    assert!(cmd.is_done());
    cmd.expect_no_effect_or_events();
}

#[test]
fn dropping_a_repeating_timer_handle_while_it_ticks_keeps_it_ticking() {
    let app = Timer;
    let mut model = Model::default();

    let mut cmd = app.update(Event::StartEvery, &mut model);
    let mut request = cmd.expect_one_effect().expect_notify_every();
    let timer_id = request.operation.id;

    request.resolve(timer_id).expect("effect should resolve");
    let event = cmd.expect_one_event();
    assert!(matches!(event, Event::Tick(TimerOutcome::Completed(_))));

    // drop the handle between two ticks
    drop(model.handle.take());
    cmd.expect_no_effect_or_events();
    assert!(!cmd.is_done());

    for _ in 0..2 {
        request.resolve(timer_id).expect("effect should resolve");
        let event = cmd.expect_one_event();
        assert!(matches!(event, Event::Tick(TimerOutcome::Completed(_))));
        cmd.expect_no_effect_or_events();
    }
    assert!(!cmd.is_done());

    // the app stops the ticks by aborting the command
    cmd.abort_handle().abort();
    assert!(cmd.is_done());
}

#[test]
fn a_repeating_timer_ends_when_the_shell_ends_the_stream() {
    let app = Timer;
    let mut model = Model::default();

    let mut cmd = app.update(Event::StartEvery, &mut model);
    let mut request = cmd.expect_one_effect().expect_notify_every();
    let timer_id = request.operation.id;

    request.resolve(timer_id).expect("effect should resolve");
    let event = cmd.expect_one_event();
    assert!(matches!(event, Event::Tick(TimerOutcome::Completed(_))));

    // the shell drops its side of the stream, which ends it
    drop(request);

    // the stream ends without a `Cleared` item
    cmd.expect_no_effect_or_events();
    assert!(cmd.is_done());
}

#[test]
fn dropping_a_repeating_timer_stream_releases_the_shell_request_at_once() {
    let app = Timer;
    let mut model = Model::default();

    let mut cmd = app.update(Event::StartEveryOnce, &mut model);
    let mut request = cmd.expect_one_effect().expect_notify_every();
    let timer_id = request.operation.id;

    // the consumer takes the first tick, then drops the stream
    request.resolve(timer_id).expect("effect should resolve");
    let event = cmd.expect_one_event();
    let Event::FirstTick(Some(TimerOutcome::Completed(completed))) = &event else {
        panic!("expected the first tick, got {event:?}");
    };
    assert_eq!(
        completed,
        model.handle.as_ref().expect("the handle is stored")
    );

    // the core no longer listens, so the next tick returns an error, and
    // the shell is not asked to clear the timer
    request
        .resolve(timer_id)
        .expect_err("the core no longer listens for ticks");
    cmd.expect_no_effect_or_events();
    assert!(cmd.is_done());
}

#[test]
#[should_panic(expected = "notify_every needs a period that is not zero")]
fn a_repeating_timer_with_a_zero_period_panics() {
    let _ = Time::<Effect, Event>::notify_every(StdDuration::ZERO);
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

    let notify_every = operation::NotifyEvery {
        id: TimerId(5),
        period: Duration::from_secs(2),
    };
    let serialized = serde_json::to_string(&notify_every).unwrap();
    assert_eq!(&serialized, r#"{"id":5,"period":{"nanos":2000000000}}"#);
    assert_eq!(
        serde_json::from_str::<operation::NotifyEvery>(&serialized).unwrap(),
        notify_every
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

    // `NotifyAt`, `NotifyAfter`, `NotifyEvery` and `Clear` are all answered
    // with a `TimerId`
    assert_eq!(serde_json::to_string(&TimerId(4)).unwrap(), "4");
}
