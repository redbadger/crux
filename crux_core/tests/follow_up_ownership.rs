use crux_core::{App, Command, Core, Request, capability::Operation, command::AbortHandle};

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
enum Effect {
    Fetch(Request<Fetch>),
    Save(Request<Save>),
    Subscribe(Request<Subscribe>),
    After(Request<After>),
}

impl crux_core::Effect for Effect {}

macro_rules! from_request {
    ($($op:ident),*) => {$(
        impl From<Request<$op>> for Effect {
            fn from(value: Request<$op>) -> Self {
                Self::$op(value)
            }
        }
    )*};
}
from_request!(Fetch, Save, Subscribe, After);

#[derive(Debug)]
enum Event {
    Start,
    E1,
    E2,
    Fetched,
    Init,
    Tick,
    Done,
}

#[derive(Default)]
struct Model {
    handle: Option<AbortHandle>,
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
    type ViewModel = ();
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
            (Scenario::AbortThenFollowUp, Event::Start) => {
                let x = Command::all([
                    Command::request_from_shell(Fetch).then_send(|()| Event::Fetched)
                ]);
                model.handle = Some(x.abort_handle());
                x
            }
            (Scenario::AbortThenFollowUp, Event::Fetched) => {
                if let Some(handle) = model.handle.take() {
                    handle.abort();
                }
                Command::notify_shell(Save).into()
            }
            (Scenario::LongLivedThen, Event::Start) => Command::event(Event::Init)
                .then(Command::request_from_shell(After).then_send(|()| Event::Done)),
            (Scenario::LongLivedThen, Event::Init) => {
                Command::stream_from_shell(Subscribe).then_send(|()| Event::Tick)
            }
            _ => Command::done(),
        }
    }

    fn view(&self, _model: &Model) {}
}

/// When a command is aborted while it still has events queued, the follow-up
/// command for a queued event is spawned onto the aborted command. Its task
/// channel is already closed, so `Core::process` panics. Dropping the emitting
/// command must not panic.
#[test]
#[ignore = "finding 2: follow-up spawned onto a dropped emitter panics with 'tasks channel disconnected'"]
fn dropping_the_emitting_command_with_queued_events_does_not_panic() {
    let core = Core::new_with(TestApp(Scenario::QueuedEvents), Model::default());

    let _effects = core.process_event(Event::Start);
}

/// A follow-up command returned by an event handler is spawned onto the command
/// that emitted the event. If the handler aborts that command, the follow-up
/// is dropped with it. The follow-up must still run, because the app asked
/// for it.
#[test]
#[ignore = "finding 3: follow-up returned after aborting its emitter is spawned onto the aborted command and lost"]
fn a_follow_up_returned_after_aborting_the_emitter_still_runs() {
    let core = Core::new_with(TestApp(Scenario::AbortThenFollowUp), Model::default());

    let mut effects = core.process_event(Event::Start);
    assert_eq!(effects.len(), 1);
    let Effect::Fetch(mut request) = effects.remove(0) else {
        panic!("expected a Fetch request");
    };

    let effects = core.resolve(&mut request, ()).expect("resolve");

    assert!(
        effects.iter().any(|e| matches!(e, Effect::Save(_))),
        "expected a Save notification, got {} effects",
        effects.len()
    );
}

/// A follow-up command that is long lived (here a subscription that never
/// ends) is hosted in the emitting command. `.then` waits for that command to
/// finish, so the command chained after it never starts. It must start once
/// the event has been handled.
#[test]
#[ignore = "finding 4: .then waits for a never-ending follow-up so the next command never starts"]
fn a_long_lived_follow_up_does_not_block_then() {
    let core = Core::new_with(TestApp(Scenario::LongLivedThen), Model::default());

    let effects = core.process_event(Event::Start);

    assert!(
        effects.iter().any(|e| matches!(e, Effect::Subscribe(_))),
        "expected the Subscribe stream request"
    );
    assert!(
        effects.iter().any(|e| matches!(e, Effect::After(_))),
        "expected the After request, got {} effects",
        effects.len()
    );
}
