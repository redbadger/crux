use crux_core::{App, Command, Core, capability::Operation, command::AbortHandle};

#[derive(Debug, PartialEq, Eq)]
struct Fetch;
impl Operation for Fetch {
    type Output = ();
}

#[derive(Debug, PartialEq, Eq)]
struct Save;
impl Operation for Save {
    type Output = ();
}

#[derive(Debug, PartialEq, Eq)]
struct Subscribe;
impl Operation for Subscribe {
    type Output = ();
}

#[derive(Debug, PartialEq, Eq)]
struct After;
impl Operation for After {
    type Output = ();
}

#[allow(dead_code)]
#[crux_core::macros::effect]
enum Effect {
    Fetch(Fetch),
    Save(Save),
    Subscribe(Subscribe),
    After(After),
}

#[derive(Debug)]
enum Event {
    Start,
    E1,
    E2,
    Fetched,
    Init,
    Tick,
    StopSubscription,
    Done,
}

#[derive(Default)]
struct Model {
    handle: Option<AbortHandle>,
    fetched: bool,
}

/// Which scenario `Start` sets up.
#[derive(Clone, Copy)]
enum Scenario {
    QueuedEvents,
    AbortThenFollowUp,
    LongLivedThen,
}

struct TestApp(Scenario);

impl App for TestApp {
    type Event = Event;
    type Model = Model;
    type ViewModel = bool;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match (self.0, event) {
            (Scenario::QueuedEvents, Event::Start) => {
                let c = Command::new(|ctx| async move {
                    ctx.send_event(Event::E1);
                    ctx.send_event(Event::E2);
                });
                let x = Command::all([
                    c,
                    Command::request_from_shell(Fetch).then_send(|()| Event::E2),
                ]);
                model.handle = Some(x.abort_handle());
                x
            }
            (Scenario::QueuedEvents, Event::E1) => {
                if let Some(handle) = model.handle.take() {
                    handle.abort();
                }
                Command::done()
            }
            (Scenario::QueuedEvents, Event::E2) => Command::notify_shell(Save).into(),
            (Scenario::AbortThenFollowUp, Event::Start) => {
                let x = Command::all([
                    Command::request_from_shell(Fetch).then_send(|()| Event::Fetched)
                ]);
                model.handle = Some(x.abort_handle());
                x
            }
            (Scenario::AbortThenFollowUp, Event::Fetched) => {
                model.fetched = true;
                if let Some(handle) = model.handle.take() {
                    handle.abort();
                }
                Command::notify_shell(Save).into()
            }
            (Scenario::LongLivedThen, Event::Start) => Command::event(Event::Init)
                .then(Command::request_from_shell(After).then_send(|()| Event::Done)),
            (Scenario::LongLivedThen, Event::Init) => {
                let subscription =
                    Command::stream_from_shell(Subscribe).then_send(|()| Event::Tick);
                model.handle = Some(subscription.abort_handle());
                subscription
            }
            (Scenario::LongLivedThen, Event::StopSubscription) => {
                model
                    .handle
                    .take()
                    .expect("subscription has started")
                    .abort();
                Command::done()
            }
            _ => Command::done(),
        }
    }

    fn view(&self, model: &Model) -> bool {
        model.fetched
    }
}

/// Queued events survive cancellation, and their updates can start new work.
#[test]
fn dropping_the_emitting_command_with_queued_events_does_not_panic() {
    let core = Core::new_with(TestApp(Scenario::QueuedEvents), Model::default());

    let effects = core.process_event(Event::Start);
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::Save(_))),
        "the queued event must still start Save"
    );
}

/// Aborting the emitter does not cancel the command returned by this update.
#[test]
fn a_follow_up_returned_after_aborting_the_emitter_still_runs() {
    let core = Core::new_with(TestApp(Scenario::AbortThenFollowUp), Model::default());

    let mut effects = core.process_event(Event::Start);
    assert_eq!(effects.len(), 1);
    let Effect::Fetch(mut request) = effects.remove(0) else {
        panic!("expected a Fetch request");
    };

    let effects = core.resolve(&mut request, ()).expect("resolve");

    assert!(
        core.view(),
        "the event's model update must still take effect"
    );
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::Save(_))),
        "the independent follow-up must send Save"
    );
}

/// A subscription started by a later update does not block the emitter's `.then`.
#[test]
fn follow_up_subscription_does_not_block_then() {
    let core = Core::new_with(TestApp(Scenario::LongLivedThen), Model::default());
    let effects = core.process_event(Event::Start);
    assert_eq!(effects.len(), 2);
    let mut subscription = None;
    let mut after = None;
    for effect in effects {
        match effect {
            Effect::Subscribe(request) => subscription = Some(request),
            Effect::After(request) => after = Some(request),
            _ => panic!("unexpected effect"),
        }
    }
    let mut subscription = subscription.expect("subscription must start");
    let mut after = after.expect(".then must start without waiting for subscription");
    assert!(core.resolve(&mut after, ()).unwrap().is_empty());
    assert!(core.resolve(&mut subscription, ()).unwrap().is_empty());
    assert!(core.process_event(Event::StopSubscription).is_empty());
    drop(subscription);
}
