use crux_core::{App, Command, Core, capability::Operation, command::AbortHandle};

#[derive(Debug, PartialEq, Eq)]
struct Subscribe;
impl Operation for Subscribe {
    type Output = ();
}

#[derive(Debug, PartialEq, Eq)]
struct Save;
impl Operation for Save {
    type Output = ();
}

#[allow(dead_code)]
#[crux_core::macros::effect]
enum Effect {
    Subscribe(Subscribe),
    Save(Save),
}

#[derive(Debug)]
enum Event {
    Start,
    ParentEmitted,
    ChildEmitted,
    Tick,
}

#[derive(Default)]
struct Model {
    parent: Option<AbortHandle>,
    sibling: Option<AbortHandle>,
}

/// `update` for `ChildEmitted` always aborts the parent command. This flag
/// selects if `update` aborts the sibling command first.
struct TestApp {
    abort_sibling_first: bool,
}

impl App for TestApp {
    type Event = Event;
    type Model = Model;
    type ViewModel = ();
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::Start => {
                let parent = Command::event(Event::ParentEmitted);
                model.parent = Some(parent.abort_handle());
                parent
            }
            // The follow-up command of the parent: a sibling that stays
            // active, and a command that emits `ChildEmitted`.
            Event::ParentEmitted => {
                let sibling = Command::stream_from_shell(Subscribe).then_send(|()| Event::Tick);
                model.sibling = Some(sibling.abort_handle());
                Command::all([sibling, Command::event(Event::ChildEmitted)])
            }
            // `Save` is a follow-up command in the chain of the parent, so
            // the abort of the parent must cancel it.
            Event::ChildEmitted => {
                if self.abort_sibling_first {
                    model.sibling.take().expect("sibling started").abort();
                }
                model.parent.take().expect("parent started").abort();
                Command::notify_shell(Save).into()
            }
            Event::Tick => Command::done(),
        }
    }

    fn view(&self, _model: &Model) {}
}

fn sends_save(effects: &[Effect]) -> bool {
    effects
        .iter()
        .any(|effect| matches!(effect, Effect::Save(_)))
}

/// The control case. The abort of the parent command cancels the follow-up
/// command that the same `update` call returns.
#[test]
fn aborting_the_parent_cancels_the_returned_follow_up() {
    let core = Core::new_with(
        TestApp {
            abort_sibling_first: false,
        },
        Model::default(),
    );

    let effects = core.process_event(Event::Start);

    assert!(!sends_save(&effects), "the cancelled follow-up sent Save");
}

/// The abort of the sibling schedules the host of the event owner before the
/// host of the parent. The root then polls the new follow-up command before
/// the cancellation from the parent reaches it, and `Save` goes to the shell.
/// The result must not depend on the order of the two aborts.
#[test]
#[ignore = "finding 2: a second abort in the same update lets the cancelled follow-up send its effect"]
fn aborting_a_sibling_first_still_cancels_the_returned_follow_up() {
    let core = Core::new_with(
        TestApp {
            abort_sibling_first: true,
        },
        Model::default(),
    );

    let effects = core.process_event(Event::Start);

    assert!(!sends_save(&effects), "the cancelled follow-up sent Save");
}
