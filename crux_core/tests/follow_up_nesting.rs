//! Kept in its own test binary because the failure is a stack overflow,
//! which aborts the whole process.

use crux_core::{App, Command, Core, Request, capability::Operation};

#[derive(Default)]
struct TickApp;

#[derive(Debug)]
enum Event {
    Tick,
}

#[derive(Debug, PartialEq, Eq)]
struct Op;

impl Operation for Op {
    type Output = ();
}

enum Effect {
    Op(Request<Op>),
}

impl crux_core::Effect for Effect {}

impl From<Request<Op>> for Effect {
    fn from(value: Request<Op>) -> Self {
        Self::Op(value)
    }
}

impl App for TickApp {
    type Event = Event;
    type Model = ();
    type ViewModel = ();
    type Effect = Effect;

    fn update(&self, event: Event, _model: &mut ()) -> Command<Effect, Event> {
        match event {
            Event::Tick => Command::request_from_shell(Op).then_send(|()| Event::Tick),
        }
    }

    fn view(&self, _model: &()) {}
}

/// Each follow-up command is spawned inside the command that emitted the event,
/// so a command that re-arms itself on every tick nests one level deeper each
/// time. A long-running loop of this kind overflows the stack. The loop must
/// run in constant stack space.
#[test]
#[ignore = "finding 1: self-re-arming follow-up loop nests commands and overflows the stack"]
fn a_self_re_arming_follow_up_loop_does_not_nest() {
    let core = Core::new_with(TickApp, ());

    let mut effects = core.process_event(Event::Tick);

    for i in 0..5_000 {
        assert_eq!(effects.len(), 1, "tick {i}");
        let Effect::Op(mut request) = effects.remove(0);
        effects = core.resolve(&mut request, ()).expect("resolve");
    }

    assert_eq!(effects.len(), 1);
}
