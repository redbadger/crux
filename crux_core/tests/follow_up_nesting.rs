//! Kept in its own test binary because the failure is a stack overflow,
//! which aborts the whole process.

use crux_core::{App, Command, Core, capability::Operation};

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

#[crux_core::macros::effect]
enum Effect {
    Op(Op),
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

/// A command that re-arms itself on every tick must run and tear down without
/// consuming stack space in proportion to the number of ticks.
#[test]
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
