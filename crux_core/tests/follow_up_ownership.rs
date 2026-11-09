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

/// Queued events can outlive their emitting command. Rejecting their child
/// trackers must cancel follow-up work without panicking.
#[test]
fn dropping_the_emitting_command_with_queued_events_does_not_panic() {
    let core = Core::new_with(TestApp(Scenario::QueuedEvents), Model::default());

    let effects = core.process_event(Event::Start);
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Save(_))),
        "follow-up work from the cancelled emitter must not send Save"
    );
}

/// Follow-up work belongs to the emitting command's lifetime, even when the
/// event handler aborts that lifetime before returning the follow-up.
#[test]
fn a_follow_up_returned_after_aborting_the_emitter_is_cancelled() {
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
        effects.is_empty(),
        "the cancelled follow-up must not send Save"
    );
}

/// `.then` waits for the follow-up subscription's command to finish. Aborting
/// just that subscription releases the continuation after cancellation cleanup.
#[test]
fn then_waits_until_the_follow_up_subscription_is_aborted() {
    let core = Core::new_with(TestApp(Scenario::LongLivedThen), Model::default());

    let mut effects = core.process_event(Event::Start);
    assert_eq!(effects.len(), 1, "only the subscription should start");
    let Effect::Subscribe(mut subscription) = effects.remove(0) else {
        panic!("expected the Subscribe stream request");
    };

    // Delivering a stream item does not finish the subscription or release `.then`.
    assert!(
        core.resolve(&mut subscription, ())
            .expect("stream item")
            .is_empty()
    );

    // Keep the shell's subscription handle alive so cancellation must come from
    // the command's abort handle, rather than dropping the shell handle.
    let mut effects = core.process_event(Event::StopSubscription);
    assert_eq!(
        effects.len(),
        1,
        "aborting the subscription should release .then"
    );
    let Effect::After(mut after) = effects.remove(0) else {
        panic!("expected the After request");
    };
    assert!(
        core.resolve(&mut after, ())
            .expect("resolve After")
            .is_empty()
    );
    drop(subscription);
}
