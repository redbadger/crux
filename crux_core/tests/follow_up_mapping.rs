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

/// Emit `Emitted`, then request `After`. Handling `Emitted` requests `FollowUp`.
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

// Identity mappings preserve which requests are ready after handling Start.
#[test]
fn then_does_not_wait_for_follow_up_with_or_without_mapping() {
    for event in [
        Event::Start,
        Event::StartWithMappedEvents,
        Event::StartWithMappedEffects,
    ] {
        let core = Core::new_with(TestApp, ());
        let effects = core.process_event(event);
        let mut names = names(&effects);
        names.sort_unstable();
        assert_eq!(names, ["After", "FollowUp"]);
    }
}
