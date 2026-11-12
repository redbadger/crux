use crux_core::{App, Command, Core, capability::Operation};

#[derive(Debug, PartialEq, Eq)]
struct FollowUp;
impl Operation for FollowUp {
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
    FollowUp(FollowUp),
    After(After),
}

#[derive(Debug)]
enum Event {
    Start,
    StartWithMappedEvents,
    StartWithMappedEffects,
    Emitted,
    Done,
}

#[derive(Default)]
struct TestApp;

/// Emits `Emitted`, then requests `After`. The follow-up command for `Emitted`
/// requests `FollowUp`, so `After` must not start before the shell resolves
/// `FollowUp`.
fn emit_then_after() -> Command<Effect, Event> {
    Command::event(Event::Emitted)
        .then(Command::request_from_shell(After).then_send(|()| Event::Done))
}

impl App for TestApp {
    type Event = Event;
    type Model = ();
    type ViewModel = ();
    type Effect = Effect;

    fn update(&self, event: Event, _model: &mut ()) -> Command<Effect, Event> {
        match event {
            Event::Start => emit_then_after(),
            Event::StartWithMappedEvents => emit_then_after().map_event(|event| event),
            Event::StartWithMappedEffects => emit_then_after().map_effect(|effect| effect),
            Event::Emitted => Command::request_from_shell(FollowUp).then_send(|()| Event::Done),
            Event::Done => Command::done(),
        }
    }

    fn view(&self, _model: &()) {}
}

fn names(effects: &[Effect]) -> Vec<&'static str> {
    effects
        .iter()
        .map(|effect| match effect {
            Effect::FollowUp(_) => "FollowUp",
            Effect::After(_) => "After",
        })
        .collect()
}

/// The control case. `.then` waits for the follow-up command of the event
/// that the first command emitted.
#[test]
fn then_waits_for_the_follow_up() {
    let core = Core::new_with(TestApp, ());

    assert_eq!(names(&core.process_event(Event::Start)), ["FollowUp"]);
}

/// `map_event` replaces the owner of each event with the mapped command, so
/// the command inside the map does not wait for the follow-up command. A
/// mapped command must behave the same as the control case.
#[test]
#[ignore = "finding 1: .then inside map_event does not wait for the follow-up command"]
fn then_inside_map_event_waits_for_the_follow_up() {
    let core = Core::new_with(TestApp, ());

    assert_eq!(
        names(&core.process_event(Event::StartWithMappedEvents)),
        ["FollowUp"]
    );
}

/// `map_effect` replaces the owner of each event in the same way.
#[test]
#[ignore = "finding 1: .then inside map_effect does not wait for the follow-up command"]
fn then_inside_map_effect_waits_for_the_follow_up() {
    let core = Core::new_with(TestApp, ());

    assert_eq!(
        names(&core.process_event(Event::StartWithMappedEffects)),
        ["FollowUp"]
    );
}
