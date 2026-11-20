//! Core processes queued events and runs each update's command independently.
use crux_core::{App, Command, Core, Request, capability::Operation, command::AbortHandle};

struct Work(&'static str);
impl Operation for Work {
    type Output = ();
}

#[crux_core::macros::effect]
enum Effect {
    Work(Work),
}

enum Event {
    Start,
    First,
    Second,
    Abort,
    Completed(&'static str),
}

#[derive(Default)]
struct Model {
    abort: Option<AbortHandle>,
    handled: Vec<&'static str>,
}

type Update = dyn Fn(Event, &mut Model) -> Command<Effect, Event> + Send + Sync;
struct TestApp(Box<Update>);
impl App for TestApp {
    type Event = Event;
    type Effect = Effect;
    type Model = Model;
    type ViewModel = Vec<&'static str>;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::Abort => {
                model.abort.take().unwrap().abort();
                Command::done()
            }
            Event::Completed(name) => {
                model.handled.push(name);
                Command::done()
            }
            event => (self.0)(event, model),
        }
    }

    fn view(&self, model: &Model) -> Self::ViewModel {
        model.handled.clone()
    }
}

fn core(
    update: impl Fn(Event, &mut Model) -> Command<Effect, Event> + Send + Sync + 'static,
) -> Core<TestApp> {
    Core::new_with(TestApp(Box::new(update)), Model::default())
}

fn take_request(effects: &mut Vec<Effect>, name: &str) -> Request<Work> {
    let index = effects
        .iter()
        .position(|Effect::Work(request)| request.operation.0 == name)
        .unwrap();
    let Effect::Work(request) = effects.remove(index);
    request
}

#[test]
fn queued_events_are_processed_even_when_an_update_aborts_the_emitter() {
    let core = core(|event, model| match event {
        Event::Start => {
            let command = Command::new(|ctx| async move {
                ctx.send_event(Event::First);
                ctx.send_event(Event::Second);
                ctx.request_from_shell(Work("emitter")).await;
            });
            model.abort = Some(command.abort_handle());
            command
        }
        Event::First => {
            model.handled.push("first");
            model.abort.take().unwrap().abort();
            Command::done()
        }
        Event::Second => {
            model.handled.push("second");
            Command::request_from_shell(Work("second"))
                .then_send(|()| Event::Completed("second done"))
        }
        _ => unreachable!(),
    });
    let mut effects = core.process_event(Event::Start);
    assert_eq!(core.view(), ["first", "second"]);
    assert_eq!(effects.len(), 1);
    let mut second = take_request(&mut effects, "second");
    assert!(core.resolve(&mut second, ()).unwrap().is_empty());
    assert_eq!(core.view(), ["first", "second", "second done"]);
}

#[test]
fn updates_and_command_continuations_run_concurrently() {
    for mapping in ["none", "events", "effects"] {
        for subscription in [false, true] {
            let core = core(move |event, _| match event {
                Event::Start => {
                    let command = Command::event(Event::First).then(
                        Command::request_from_shell(Work("continuation"))
                            .then_send(|()| Event::Completed("continuation")),
                    );
                    match mapping {
                        "none" => command,
                        "events" => command.map_event(|event| event),
                        "effects" => command.map_effect(|effect| effect),
                        _ => unreachable!(),
                    }
                }
                Event::First if subscription => Command::stream_from_shell(Work("update"))
                    .then_send(|()| Event::Completed("update")),
                Event::First => Command::request_from_shell(Work("update"))
                    .then_send(|()| Event::Completed("update")),
                _ => unreachable!(),
            });
            let mut effects = core.process_event(Event::Start);
            assert_eq!(effects.len(), 2);
            let mut continuation = take_request(&mut effects, "continuation");
            let mut update = take_request(&mut effects, "update");
            assert!(core.resolve(&mut continuation, ()).unwrap().is_empty());
            assert_eq!(core.view(), ["continuation"]);
            assert!(core.resolve(&mut update, ()).unwrap().is_empty());
            assert_eq!(core.view(), ["continuation", "update"]);
        }
    }
}

#[test]
fn commands_from_separate_updates_have_independent_lifetimes() {
    let core = core(|event, model| match event {
        Event::Start => {
            let command = Command::new(|ctx| async move {
                ctx.send_event(Event::First);
                ctx.request_from_shell(Work("emitter")).await;
                ctx.send_event(Event::Completed("emitter"));
            });
            model.abort = Some(command.abort_handle());
            command
        }
        Event::First => {
            Command::request_from_shell(Work("update")).then_send(|()| Event::Completed("update"))
        }
        _ => unreachable!(),
    });
    let mut effects = core.process_event(Event::Start);
    assert_eq!(effects.len(), 2);
    let mut emitter = take_request(&mut effects, "emitter");
    let mut update = take_request(&mut effects, "update");
    assert!(core.process_event(Event::Abort).is_empty());
    assert!(core.resolve(&mut emitter, ()).unwrap().is_empty());
    assert_eq!(core.view(), Vec::<&str>::new());
    assert!(core.resolve(&mut update, ()).unwrap().is_empty());
    assert_eq!(core.view(), ["update"]);
}
