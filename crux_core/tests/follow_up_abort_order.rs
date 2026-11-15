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

/// Handling `ChildEmitted` aborts the command started by `Start`. This flag
/// selects whether the subscription is also aborted before that command.
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
            // Start a subscription alongside a command that emits `ChildEmitted`.
            Event::ParentEmitted => {
                let sibling = Command::stream_from_shell(Subscribe).then_send(|()| Event::Tick);
                model.sibling = Some(sibling.abort_handle());
                Command::all([sibling, Command::event(Event::ChildEmitted)])
            }
            // Stop the earlier work and request a save.
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

/// Neither abort order cancels a command returned by a later update.
#[test]
fn aborting_the_emitter_does_not_cancel_the_returned_command() {
    for abort_sibling_first in [false, true] {
        let core = Core::new_with(
            TestApp {
                abort_sibling_first,
            },
            Model::default(),
        );
        let effects = core.process_event(Event::Start);
        assert!(
            sends_save(&effects),
            "Save must run regardless of abort order"
        );
    }
}
