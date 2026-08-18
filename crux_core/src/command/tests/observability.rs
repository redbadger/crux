use std::sync::{Arc, Mutex};
use std::task::Poll;

use futures::{FutureExt as _, StreamExt as _, future::poll_fn};

use crate::{
    App, Command, Core, CoreRuntimeObserver, OperationKind, Request, RuntimeContextId,
    capability::Operation, command::AbortHandle,
};
use Observation::*;
use OperationKind::{Notify, Request as RequestKind, Stream};

struct Op;
impl Operation for Op {
    type Output = ();
}

// Disabling metadata access lets the unobserved test catch accidental callbacks.
struct Effect(Request<Op>, bool);
impl From<Request<Op>> for Effect {
    fn from(request: Request<Op>) -> Self {
        Self(request, true)
    }
}
impl crate::Effect for Effect {
    fn name(&self) -> &'static str {
        assert!(self.1);
        "Op"
    }
    fn kind(&self) -> OperationKind {
        assert!(self.1);
        self.0.handle.kind()
    }
}

#[derive(Clone, Copy)]
enum Event {
    Start,
    FollowUp,
    Done,
    Abort,
}
impl crate::EventName for Event {
    fn event_name(&self) -> &'static str {
        match self {
            Self::Start => "Start",
            Self::FollowUp => "FollowUp",
            Self::Done => "Done",
            Self::Abort => "Abort",
        }
    }
}

type TestCommand = Command<Effect, Event>;
struct TestApp(Box<dyn Fn(Event) -> TestCommand + Send + Sync>);
impl App for TestApp {
    type Event = Event;
    type Effect = Effect;
    type Model = Option<AbortHandle>;
    type ViewModel = ();

    fn update(&self, event: Event, model: &mut Self::Model) -> TestCommand {
        if matches!(event, Event::Abort) {
            model.take().unwrap().abort();
            return Command::done();
        }
        let save_abort = matches!(event, Event::Start);
        let command = (self.0)(event);
        if save_abort {
            *model = Some(command.abort_handle());
        }
        command
    }
    fn view(&self, _: &Self::Model) {}
}

fn core(update: impl Fn(Event) -> TestCommand + Send + Sync + 'static) -> Core<TestApp> {
    Core::new_with(TestApp(Box::new(update)), None)
}

fn follow_up(event: Event) -> TestCommand {
    match event {
        Event::Start => Command::event(Event::FollowUp),
        Event::FollowUp => Command::request_from_shell(Op).then_send(|()| Event::Done),
        Event::Done => Command::done(),
        Event::Abort => unreachable!("handled by TestApp"),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Observation {
    CommandStarted(RuntimeContextId, RuntimeContextId, &'static str),
    CommandEnded(RuntimeContextId),
    EffectStarted(RuntimeContextId, RuntimeContextId, OperationKind),
    EffectEnded(RuntimeContextId),
    Item(RuntimeContextId),
    Notification(RuntimeContextId),
    WorkDropped(&'static str),
    ThenRan,
}

#[derive(Clone, Default)]
struct Recorder(Arc<Mutex<Vec<Observation>>>);
impl Recorder {
    fn push(&self, observation: Observation) {
        self.0.lock().unwrap().push(observation);
    }
    fn take(&self) -> Vec<Observation> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
    fn count(&self, observation: Observation) -> usize {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter(|o| **o == observation)
            .count()
    }
    fn command(&self, name: &str) -> RuntimeContextId {
        self.0
            .lock()
            .unwrap()
            .iter()
            .find_map(|o| match o {
                CommandStarted(id, _, label) if *label == name => Some(*id),
                _ => None,
            })
            .expect("command started")
    }
    fn effect(&self, parent: RuntimeContextId) -> RuntimeContextId {
        self.0
            .lock()
            .unwrap()
            .iter()
            .find_map(|o| match o {
                EffectStarted(id, owner, _) if *owner == parent => Some(*id),
                _ => None,
            })
            .expect("effect started")
    }
    fn assert_before(&self, first: Observation, second: Observation) {
        let observations = self.0.lock().unwrap().clone();
        let position = |item| observations.iter().position(|o| *o == item).unwrap();
        assert!(position(first) < position(second), "{observations:?}");
    }
}
impl CoreRuntimeObserver for Recorder {
    fn command_started(&self, id: RuntimeContextId, cause: RuntimeContextId, name: &'static str) {
        self.push(CommandStarted(id, cause, name));
    }
    fn command_ended(&self, id: RuntimeContextId) {
        self.push(CommandEnded(id));
    }
    fn effect_started(
        &self,
        id: RuntimeContextId,
        parent: RuntimeContextId,
        name: &'static str,
        kind: OperationKind,
    ) {
        assert_eq!(name, "Op");
        self.push(EffectStarted(id, parent, kind));
    }
    fn effect_ended(&self, id: RuntimeContextId) {
        self.push(EffectEnded(id));
    }
    fn stream_item(&self, id: RuntimeContextId) {
        self.push(Item(id));
    }
    fn notification(&self, parent: RuntimeContextId, name: &'static str) {
        assert_eq!(name, "Op");
        self.push(Notification(parent));
    }
}

struct DropMark(Recorder, &'static str);
impl Drop for DropMark {
    fn drop(&mut self) {
        self.0.push(WorkDropped(self.1));
    }
}

#[test]
fn command_names_and_causal_links_survive_the_emitter_ending() {
    let log = Recorder::default();
    let core = core(follow_up).with_runtime_observer(log.clone());
    let mut effects = core.process_event(Event::Start);
    assert_eq!(effects.len(), 1);
    let root = log.command("Start");
    let child = log.command("FollowUp");
    let effect = log.effect(child);
    assert_eq!(
        log.take(),
        [
            CommandStarted(root, RuntimeContextId::ROOT, "Start"),
            CommandEnded(root),
            CommandStarted(child, root, "FollowUp"),
            EffectStarted(effect, child, RequestKind),
        ]
    );
    assert_ne!(root, child);
    assert_ne!(effect, child);
    assert_ne!(effect, root);
    assert_ne!(effect, RuntimeContextId::ROOT);

    let Effect(mut request, _) = effects.remove(0);
    assert!(core.resolve(&mut request, ()).unwrap().is_empty());
    let done = log.command("Done");
    assert_eq!(
        log.take(),
        [
            EffectEnded(effect),
            CommandEnded(child),
            CommandStarted(done, child, "Done"),
            CommandEnded(done),
        ]
    );
}

#[test]
fn effect_kind_comes_from_the_handle_and_notifications_are_observed_at_dispatch() {
    // One operation type can have all three kinds: its handle determines which.
    for kind in [Notify, RequestKind, Stream] {
        let log = Recorder::default();
        let core = core(move |_| {
            Command::new(move |ctx| async move {
                match kind {
                    Notify => ctx.notify_shell(Op),
                    RequestKind => ctx.request_from_shell(Op).await,
                    Stream => {
                        let _ = ctx.stream_from_shell(Op).next().await;
                    }
                }
            })
        })
        .with_runtime_observer(log.clone());
        let effects = core.process_event(Event::Start);
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].0.handle.kind(), kind);
        let command = log.command("Start");
        let start = CommandStarted(command, RuntimeContextId::ROOT, "Start");
        if kind == Notify {
            assert_eq!(
                log.take(),
                [start, CommandEnded(command), Notification(command)]
            );
        } else {
            let effect = log.effect(command);
            assert_eq!(log.take(), [start, EffectStarted(effect, command, kind)]);
        }
    }
}

#[test]
fn composition_and_mapping_observe_each_output_once_with_its_command_identity() {
    let log = Recorder::default();
    let core = core(|event| match event {
        Event::Start => {
            let notify = || Command::<Request<Op>, ()>::notify_shell(Op).build();
            Command::all([notify(), notify().and(notify())])
                .then(Command::event(()))
                .map_effect(Effect::from)
                .map_event(|()| Event::FollowUp)
        }
        event => follow_up(event),
    })
    .with_runtime_observer(log.clone());
    let effects = core.process_event(Event::Start);
    assert_eq!(effects.len(), 4);
    let root = log.command("Start");
    let child = log.command("FollowUp");
    let effect = log.effect(child);
    assert_eq!(
        log.take(),
        [
            CommandStarted(root, RuntimeContextId::ROOT, "Start"),
            CommandEnded(root),
            CommandStarted(child, root, "FollowUp"),
            Notification(root),
            Notification(root),
            Notification(root),
            EffectStarted(effect, child, RequestKind),
        ]
    );
    for Effect(mut request, _) in effects {
        if request.handle.kind() == RequestKind {
            assert!(core.resolve(&mut request, ()).unwrap().is_empty());
        }
    }
    assert_eq!(log.count(EffectEnded(effect)), 1);
    assert_eq!(log.count(CommandEnded(child)), 1);
}

#[test]
fn unobserved_hosting_does_not_read_effect_metadata() {
    let core =
        core(|event| follow_up(event).map_effect(|Effect(request, _)| Effect(request, false)));
    let mut effects = core.process_event(Event::Start);
    assert_eq!(effects.len(), 1);
    assert!(
        core.resolve(&mut effects.remove(0).0, ())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn observation_does_not_make_then_wait_for_a_later_update() {
    for observed in [false, true] {
        let log = Recorder::default();
        let task_log = log.clone();
        let core = core(move |event| match event {
            Event::Start => {
                let log = task_log.clone();
                Command::event(Event::FollowUp).then(Command::new(move |_| async move {
                    log.push(ThenRan);
                }))
            }
            event => follow_up(event),
        });
        let core = if observed {
            core.with_runtime_observer(log.clone())
        } else {
            core
        };
        let mut effects = core.process_event(Event::Start);
        assert_eq!(effects.len(), 1);
        assert_eq!(log.count(ThenRan), 1);
        if observed {
            assert_eq!(log.count(CommandEnded(log.command("Start"))), 1);
            assert_eq!(log.count(CommandEnded(log.command("FollowUp"))), 0);
        }
        assert!(
            core.resolve(&mut effects.remove(0).0, ())
                .unwrap()
                .is_empty()
        );
        if observed {
            assert_eq!(log.count(CommandEnded(log.command("FollowUp"))), 1);
        } else {
            assert_eq!(log.take(), [ThenRan]);
        }
    }
}

#[test]
fn aborting_an_active_emitter_cleans_up_only_its_own_work() {
    let log = Recorder::default();
    let task_log = log.clone();
    let core = core(move |event| match event {
        Event::Start => {
            let guard = DropMark(task_log.clone(), "emitter");
            Command::new(move |ctx| async move {
                let _guard = guard;
                ctx.send_event(Event::FollowUp);
                ctx.request_from_shell(Op).await;
            })
        }
        event => follow_up(event),
    })
    .with_runtime_observer(log.clone());
    let effects = core.process_event(Event::Start);
    assert_eq!(effects.len(), 2);
    let root = log.command("Start");
    let child = log.command("FollowUp");
    assert_eq!(log.count(WorkDropped("emitter")), 0);
    assert!(core.process_event(Event::Abort).is_empty());
    log.assert_before(WorkDropped("emitter"), CommandEnded(root));
    assert_eq!(log.count(CommandEnded(root)), 1);
    assert_eq!(log.count(CommandEnded(child)), 0);
    for Effect(mut request, _) in effects {
        assert!(core.resolve(&mut request, ()).unwrap().is_empty());
    }
    assert_eq!(log.count(CommandEnded(root)), 1);
    assert_eq!(log.count(CommandEnded(child)), 1);
}

#[test]
fn request_ends_once_and_work_is_dropped_before_command_end() {
    for mode in ["resolve", "abort", "drop handle", "drop core"] {
        let log = Recorder::default();
        let task_log = log.clone();
        let core = core(move |_| {
            let guard = DropMark(task_log.clone(), "request");
            Command::new(move |ctx| async move {
                let _guard = guard;
                ctx.request_from_shell(Op).await;
            })
        })
        .with_runtime_observer(log.clone());
        let mut effects = core.process_event(Event::Start);
        assert_eq!(effects.len(), 1);
        let command = log.command("Start");
        let effect = log.effect(command);
        assert_eq!(log.count(EffectEnded(effect)), 0);
        assert_eq!(log.count(WorkDropped("request")), 0);
        let Effect(mut request, _) = effects.remove(0);
        match mode {
            "resolve" => {
                assert!(core.resolve(&mut request, ()).unwrap().is_empty());
                drop(core);
            }
            "abort" => {
                assert!(core.process_event(Event::Abort).is_empty());
                assert_eq!(log.count(EffectEnded(effect)), 1);
                // A late response cannot produce another end.
                assert!(core.resolve(&mut request, ()).unwrap().is_empty());
                drop(core);
            }
            "drop handle" => {
                drop(request);
                assert!(core.process().is_empty());
                drop(core);
            }
            "drop core" => {
                drop(core);
                drop(request);
            }
            _ => unreachable!(),
        }
        assert_eq!(log.count(EffectEnded(effect)), 1, "{mode}");
        assert_eq!(log.count(CommandEnded(command)), 1, "{mode}");
        assert_eq!(log.count(Item(effect)), 0);
        log.assert_before(EffectEnded(effect), CommandEnded(command));
        log.assert_before(WorkDropped("request"), CommandEnded(command));
    }
}

#[test]
fn unpolled_host_drops_work_before_reporting_command_end() {
    let log = Recorder::default();
    let guard = DropMark(log.clone(), "unpolled");
    let command: TestCommand = Command::new(move |_| async move {
        let _guard = guard;
        std::future::pending::<()>().await;
    });
    let parent: TestCommand = Command::done();
    let context = crate::core::RuntimeObservation::<Event>::new(log.clone());
    let observation = context.start_command(&Event::Start, None);
    let id = log.command("Start");
    drop(command.host_with_observation(parent.context, observation));
    assert_eq!(
        log.take(),
        [
            CommandStarted(id, RuntimeContextId::ROOT, "Start"),
            WorkDropped("unpolled"),
            CommandEnded(id),
        ]
    );
}

#[test]
fn stream_markers_follow_consumption_not_delivery_and_end_on_drop() {
    let log = Recorder::default();
    let core = core(|_| {
        Command::new(|ctx| async move {
            let mut stream = ctx.stream_from_shell(Op);
            assert_eq!(stream.next().await, Some(()));
            ctx.request_from_shell(Op).await;
            assert_eq!(stream.next().await, Some(()));
        })
    })
    .with_runtime_observer(log.clone());
    let Effect(mut stream, _) = core.process_event(Event::Start).remove(0);
    let command = log.command("Start");
    let effect = log.effect(command);
    let Effect(mut gate, _) = core.resolve(&mut stream, ()).unwrap().remove(0);
    stream.resolve(()).unwrap();
    assert!(core.process().is_empty());
    assert_eq!(
        log.count(Item(effect)),
        1,
        "the second response is still buffered"
    );
    assert_eq!(log.count(EffectEnded(effect)), 0);
    assert!(core.resolve(&mut gate, ()).unwrap().is_empty());
    assert_eq!(log.count(Item(effect)), 2);
    assert_eq!(log.count(EffectEnded(effect)), 1);
    log.assert_before(EffectEnded(effect), CommandEnded(command));
}

#[test]
fn stream_ends_when_sender_disappears_or_command_is_aborted() {
    for abort in [false, true] {
        let log = Recorder::default();
        let core = core(|_| {
            Command::new(|ctx| async move {
                let mut stream = ctx.stream_from_shell(Op);
                while stream.next().await.is_some() {}
            })
        })
        .with_runtime_observer(log.clone());
        let effects = core.process_event(Event::Start);
        let command = log.command("Start");
        let effect = log.effect(command);
        if abort {
            assert!(core.process_event(Event::Abort).is_empty());
        } else {
            drop(effects);
            assert!(core.process().is_empty());
        }
        assert_eq!(log.count(EffectEnded(effect)), 1);
        log.assert_before(EffectEnded(effect), CommandEnded(command));
    }
}

#[test]
fn abandoned_waits_are_discarded_but_notifications_are_dispatched() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    for observed in [false, true] {
        for stream in [false, true] {
            for poll_once in [false, true] {
                let log = Recorder::default();
                let forwarded = Arc::new(AtomicUsize::new(0));
                let mapped = forwarded.clone();
                let core = core(move |_| {
                    let mapped = mapped.clone();
                    Command::new(move |ctx| async move {
                        ctx.notify_shell(Op);
                        let mut wait = if stream {
                            ctx.stream_from_shell(Op)
                                .into_future()
                                .map(|_| ())
                                .left_future()
                        } else {
                            ctx.request_from_shell(Op).right_future()
                        };
                        if poll_once {
                            poll_fn(|cx| {
                                assert!(wait.poll_unpin(cx).is_pending());
                                Poll::Ready(())
                            })
                            .await;
                        }
                        drop(wait);
                    })
                    .map_effect(move |effect| {
                        mapped.fetch_add(1, Ordering::Relaxed);
                        effect
                    })
                });
                let core = if observed {
                    core.with_runtime_observer(log.clone())
                } else {
                    core
                };
                let effects = core.process_event(Event::Start);
                // Composition must forward every emitted effect, including an
                // abandoned wait. Only Core suppresses it at dispatch.
                assert_eq!(
                    forwarded.load(Ordering::Relaxed),
                    1 + usize::from(poll_once)
                );
                assert_eq!(effects.len(), 1);
                assert_eq!(effects[0].0.handle.kind(), Notify);
                if observed {
                    let command = log.command("Start");
                    assert_eq!(
                        log.take(),
                        [
                            CommandStarted(command, RuntimeContextId::ROOT, "Start"),
                            CommandEnded(command),
                            Notification(command),
                        ]
                    );
                } else {
                    assert_eq!(log.take(), []);
                }
            }
        }
    }
}

#[test]
fn cancellation_before_core_dispatch_discards_the_queued_effect() {
    for observed in [false, true] {
        let log = Recorder::default();
        let core = core(|_| {
            Command::new(|ctx| async move {
                ctx.send_event(Event::Abort);
                ctx.request_from_shell(Op).await;
            })
        });
        let core = if observed {
            core.with_runtime_observer(log.clone())
        } else {
            core
        };
        // The host has queued the effect, but Core handles Abort before its
        // final effect drain. Hosting alone must not commit dispatch.
        assert!(core.process_event(Event::Start).is_empty());
        assert!(
            !log.take()
                .iter()
                .any(|item| matches!(item, EffectStarted(..)))
        );
    }
}

#[test]
fn completed_waits_end_their_spans_while_the_wrappers_are_retained() {
    for stream in [false, true] {
        let log = Recorder::default();
        let core = core(move |_| {
            Command::new(move |ctx| async move {
                if stream {
                    let mut wait = ctx.stream_from_shell(Op);
                    while wait.next().await.is_some() {}
                    ctx.request_from_shell(Op).await;
                    drop(wait);
                } else {
                    let mut wait = ctx.request_from_shell(Op);
                    (&mut wait).await;
                    ctx.request_from_shell(Op).await;
                    drop(wait);
                }
            })
        })
        .with_runtime_observer(log.clone());
        let mut effects = core.process_event(Event::Start);
        let command = log.command("Start");
        let effect = log.effect(command);
        let Effect(mut request, _) = effects.remove(0);
        let gates = if stream {
            drop(request);
            core.process()
        } else {
            core.resolve(&mut request, ()).unwrap()
        };
        assert_eq!(gates.len(), 1);
        assert_eq!(log.count(EffectEnded(effect)), 1);
        assert_eq!(log.count(CommandEnded(command)), 0);
        drop(core);
        assert_eq!(log.count(EffectEnded(effect)), 1);
    }
}
