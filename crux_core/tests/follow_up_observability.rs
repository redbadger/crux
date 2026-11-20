//! Follow-up chains must not consume stack in proportion to their length.
//! Kept in a separate binary because a stack overflow aborts the test process.

use std::sync::{Arc, Mutex};

use crux_core::{
    App, Command, Core, CoreRuntimeObserver, RuntimeContextId, capability::Operation,
    command::AbortHandle,
};

const DEPTH: usize = 5_000;

struct TickApp;

#[derive(crux_core::macros::EventName)]
enum Event {
    Tick(usize),
    Abort,
}

struct Tick;

impl Operation for Tick {
    type Output = ();
}

#[crux_core::macros::effect]
enum Effect {
    Tick(Tick),
}

impl App for TickApp {
    type Event = Event;
    type Model = Option<AbortHandle>;
    type ViewModel = ();
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Self::Model) -> Command<Effect, Event> {
        match event {
            Event::Tick(0) => Command::done(),
            Event::Tick(remaining) => {
                let command = Command::request_from_shell(Tick)
                    .then_send(move |()| Event::Tick(remaining - 1));
                if model.is_none() {
                    *model = Some(command.abort_handle());
                }
                command
            }
            Event::Abort => {
                model.take().expect("tick chain has started").abort();
                Command::done()
            }
        }
    }

    fn view(&self, _: &Self::Model) {}
}

#[derive(Default)]
struct Observations {
    starts: Vec<(RuntimeContextId, RuntimeContextId)>,
    ends: Vec<RuntimeContextId>,
}

#[derive(Clone, Default)]
struct Observer(Arc<Mutex<Observations>>);

impl CoreRuntimeObserver for Observer {
    fn command_started(&self, id: RuntimeContextId, parent: RuntimeContextId, _name: &'static str) {
        self.0.lock().unwrap().starts.push((id, parent));
    }

    fn command_ended(&self, id: RuntimeContextId) {
        self.0.lock().unwrap().ends.push(id);
    }
}

fn resolve_tick(core: &Core<TickApp>, mut effects: Vec<Effect>) -> Vec<Effect> {
    assert_eq!(effects.len(), 1);
    let Effect::Tick(mut request) = effects.remove(0);
    core.resolve(&mut request, ()).expect("resolve tick")
}

#[test]
fn a_deep_follow_up_chain_completes_without_recursive_polling_or_waking() {
    let observer = Observer::default();
    let core = Core::new_with(TickApp, None).with_runtime_observer(observer.clone());
    let mut effects = core.process_event(Event::Tick(DEPTH));
    for _ in 0..DEPTH {
        effects = resolve_tick(&core, effects);
    }
    assert!(effects.is_empty());

    let observations = observer.0.lock().unwrap();
    assert_eq!(observations.starts.len(), DEPTH + 1);
    assert_eq!(observations.starts[0].1, RuntimeContextId::ROOT);
    for pair in observations.starts.windows(2) {
        assert_eq!(pair[1].1, pair[0].0);
    }
    let expected: Vec<_> = observations.starts.iter().map(|(id, _)| *id).collect();
    assert_eq!(observations.ends, expected);
    drop(observations);
}

#[test]
fn aborting_the_emitter_does_not_end_an_active_follow_up_command() {
    let observer = Observer::default();
    let core = Core::new_with(TickApp, None).with_runtime_observer(observer.clone());
    let mut effects = core.process_event(Event::Tick(DEPTH + 1));
    for _ in 0..DEPTH {
        effects = resolve_tick(&core, effects);
    }
    // The active command belongs to a separate update call from the emitter.
    assert_eq!(effects.len(), 1);
    assert!(core.process_event(Event::Abort).is_empty());

    {
        let observations = observer.0.lock().unwrap();
        assert_eq!(observations.starts.len(), DEPTH + 2);
        let abort_event = observations.starts.last().unwrap().0;
        let mut expected: Vec<_> = observations.starts[..DEPTH]
            .iter()
            .map(|(id, _)| *id)
            .collect();
        expected.push(abort_event);
        assert_eq!(observations.ends, expected);
    }
    assert!(resolve_tick(&core, effects).is_empty());
    let observations = observer.0.lock().unwrap();
    let mut expected: Vec<_> = observations.starts[..DEPTH]
        .iter()
        .map(|(id, _)| *id)
        .collect();
    expected.push(observations.starts[DEPTH + 1].0);
    expected.push(observations.starts[DEPTH].0);
    expected.push(observations.starts.last().unwrap().0);
    assert_eq!(observations.ends, expected);
}

#[test]
fn dropping_core_with_a_deep_chain_reports_each_event_end_once() {
    let observer = Observer::default();
    let core = Core::new_with(TickApp, None).with_runtime_observer(observer.clone());
    let mut effects = core.process_event(Event::Tick(DEPTH + 1));
    for _ in 0..DEPTH {
        effects = resolve_tick(&core, effects);
    }
    assert_eq!(effects.len(), 1);
    drop(core);

    // Completed commands have already closed; teardown closes the active leaf.
    let observations = observer.0.lock().unwrap();
    let expected: Vec<_> = observations.starts.iter().map(|(id, _)| id.0).collect();
    let actual: Vec<_> = observations.ends.iter().map(|id| id.0).collect();
    drop(observations);
    assert_eq!(actual, expected);
}
