use std::sync::{Arc, Mutex};

use crate::{Command, Core, CoreRuntimeObserver, Request, RuntimeContextId, capability::Operation};

#[derive(Default)]
struct TestApp;

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Start,
    FollowUp,
    Done,
    Emit(crate::OperationKind),
    Composed,
}

#[derive(Debug, PartialEq, Eq)]
enum Op {
    FollowUp,
}

impl Operation for Op {
    type Output = ();
}

enum Effect {
    Op(Request<Op>),
}

impl crate::Effect for Effect {
    fn name(&self) -> &'static str {
        match self {
            Self::Op(_) => "Op",
        }
    }

    fn kind(&self) -> crate::OperationKind {
        match self {
            Self::Op(request) => request.handle.kind(),
        }
    }
}

impl From<Request<Op>> for Effect {
    fn from(value: Request<Op>) -> Self {
        Self::Op(value)
    }
}

impl crate::App for TestApp {
    type Event = Event;
    type Model = ();
    type ViewModel = ();
    type Effect = Effect;

    fn update(
        &self,
        event: Self::Event,
        _model: &mut Self::Model,
    ) -> Command<Self::Effect, Self::Event> {
        match event {
            Event::Start => Command::new(|ctx| async move {
                ctx.send_event(Event::FollowUp);
            }),
            Event::FollowUp => {
                Command::request_from_shell(Op::FollowUp).then_send(|()| Event::Done)
            }
            Event::Done => Command::done(),
            Event::Composed => {
                let notify = || Command::notify_shell(Op::FollowUp).build();
                Command::all([notify(), notify().and(notify())])
                    .then(Command::event(Event::FollowUp))
                    .map_effect(|effect| effect)
                    .map_event(|event| event)
            }
            Event::Emit(kind) => Command::new(move |ctx| async move {
                use futures::StreamExt as _;
                match kind {
                    crate::OperationKind::Notify => ctx.notify_shell(Op::FollowUp),
                    crate::OperationKind::Request => ctx.request_from_shell(Op::FollowUp).await,
                    crate::OperationKind::Stream => {
                        let _ = ctx.stream_from_shell(Op::FollowUp).next().await;
                    }
                }
            }),
        }
    }

    fn view(&self, _model: &Self::Model) -> Self::ViewModel {}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeObservation {
    EventStarted {
        context_id: RuntimeContextId,
        parent_context_id: RuntimeContextId,
    },
    EventEnded {
        context_id: RuntimeContextId,
    },
    EffectStarted {
        context_id: RuntimeContextId,
        parent_context_id: RuntimeContextId,
    },
    Notification {
        parent_context_id: RuntimeContextId,
    },
}

#[derive(Clone, Default)]
struct TestObserver {
    observations: Arc<Mutex<Vec<RuntimeObservation>>>,
}

impl TestObserver {
    fn observations(&self) -> Vec<RuntimeObservation> {
        self.observations
            .lock()
            .expect("observations lock poisoned")
            .clone()
    }
}

impl CoreRuntimeObserver for TestObserver {
    fn event_started(
        &self,
        context_id: RuntimeContextId,
        parent_context_id: RuntimeContextId,
        _name: &'static str,
    ) {
        self.observations
            .lock()
            .expect("observations lock poisoned")
            .push(RuntimeObservation::EventStarted {
                context_id,
                parent_context_id,
            });
    }

    fn event_ended(&self, context_id: RuntimeContextId) {
        self.observations
            .lock()
            .expect("observations lock poisoned")
            .push(RuntimeObservation::EventEnded { context_id });
    }

    fn effect_started(
        &self,
        context_id: RuntimeContextId,
        parent_context_id: RuntimeContextId,
        _name: &'static str,
    ) {
        self.observations
            .lock()
            .expect("observations lock poisoned")
            .push(RuntimeObservation::EffectStarted {
                context_id,
                parent_context_id,
            });
    }

    fn notification(&self, parent_context_id: RuntimeContextId, _name: &'static str) {
        self.observations
            .lock()
            .expect("observations lock poisoned")
            .push(RuntimeObservation::Notification { parent_context_id });
    }
}

#[test]
fn runtime_observer_tracks_event_contexts_and_effects() {
    let observer = TestObserver::default();
    let core = Core::new_with(TestApp, ()).with_runtime_observer(observer.clone());

    let mut effects = core.process_event(Event::Start);
    assert_eq!(effects.len(), 1);

    let observations = observer.observations();
    let root_runtime_context_id = observations
        .iter()
        .find_map(|observation| match observation {
            RuntimeObservation::EventStarted {
                context_id,
                parent_context_id: RuntimeContextId::ROOT,
            } => Some(*context_id),
            _ => None,
        })
        .expect("root event should start");

    let follow_up_runtime_context_id = observations
        .iter()
        .find_map(|observation| match observation {
            RuntimeObservation::EventStarted {
                context_id,
                parent_context_id,
            } if *parent_context_id == root_runtime_context_id => Some(*context_id),
            _ => None,
        })
        .expect("follow-up event should be parented by root event");

    let effect_runtime_context_id = observations
        .iter()
        .find_map(|observation| match observation {
            RuntimeObservation::EffectStarted {
                context_id,
                parent_context_id,
            } if *parent_context_id == follow_up_runtime_context_id => Some(*context_id),
            _ => None,
        })
        .expect("effect should be parented by follow-up event");

    assert_ne!(effect_runtime_context_id, RuntimeContextId::ROOT);

    let Effect::Op(mut request) = effects.remove(0);
    assert_eq!(request.operation, Op::FollowUp);

    let effects = core.resolve(&mut request, ()).unwrap();
    assert!(effects.is_empty());

    let observations = observer.observations();
    assert!(observations.contains(&RuntimeObservation::EventEnded {
        context_id: root_runtime_context_id,
    }));
    assert!(observations.contains(&RuntimeObservation::EventEnded {
        context_id: follow_up_runtime_context_id,
    }));
}

#[test]
fn core_observes_the_effect_kind() {
    use crate::{Effect as _, OperationKind};

    // The same operation has no static KIND; classification must come from
    // the actual handle.
    for kind in [
        OperationKind::Notify,
        OperationKind::Request,
        OperationKind::Stream,
    ] {
        let observer = TestObserver::default();
        let core = Core::new_with(TestApp, ()).with_runtime_observer(observer.clone());
        let effects = core.process_event(Event::Emit(kind));
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].kind(), kind);

        let observations = observer.observations();
        let notifications = observations
            .iter()
            .filter(|o| matches!(o, RuntimeObservation::Notification { .. }))
            .count();
        let spans = observations
            .iter()
            .filter(|o| matches!(o, RuntimeObservation::EffectStarted { .. }))
            .count();
        assert_eq!(
            (notifications, spans),
            if kind == OperationKind::Notify {
                (1, 0)
            } else {
                (0, 1)
            }
        );
    }
}

#[test]
fn notification_is_observed_before_its_event_ends() {
    let observer = TestObserver::default();
    let core = Core::new_with(TestApp, ()).with_runtime_observer(observer.clone());
    assert_eq!(
        core.process_event(Event::Emit(crate::OperationKind::Notify))
            .len(),
        1
    );
    let observations = observer.observations();
    let [
        RuntimeObservation::EventStarted { context_id, .. },
        RuntimeObservation::Notification { parent_context_id },
        RuntimeObservation::EventEnded { context_id: ended },
    ] = observations.as_slice()
    else {
        panic!("unexpected observation order: {observations:?}");
    };
    assert_eq!(context_id, parent_context_id);
    assert_eq!(context_id, ended);
}

#[test]
fn composition_inherits_identity_and_nested_event_outputs_are_observed_once() {
    let observer = TestObserver::default();
    let core = Core::new_with(TestApp, ()).with_runtime_observer(observer.clone());
    let effects = core.process_event(Event::Composed);
    assert_eq!(effects.len(), 4);
    let observations = observer.observations();
    let RuntimeObservation::EventStarted {
        context_id: root, ..
    } = observations[0]
    else {
        panic!("expected root event start");
    };
    assert_eq!(
        observations
            .iter()
            .filter(|o| matches!(o,
                RuntimeObservation::Notification { parent_context_id } if *parent_context_id == root
            ))
            .count(),
        3
    );
    let child = observations
        .iter()
        .find_map(|o| match o {
            RuntimeObservation::EventStarted {
                context_id,
                parent_context_id,
            } if *parent_context_id == root => Some(*context_id),
            _ => None,
        })
        .expect("follow-up event should belong to the composed event");
    let effect_starts: Vec<_> = observations
        .iter()
        .filter_map(|o| match o {
            RuntimeObservation::EffectStarted {
                parent_context_id, ..
            } => Some(*parent_context_id),
            _ => None,
        })
        .collect();
    assert_eq!(effect_starts, [child]);
    assert!(
        !observations
            .iter()
            .any(|o| matches!(o, RuntimeObservation::EventEnded { .. }))
    );

    for Effect::Op(mut request) in effects {
        if request.handle.kind() == crate::OperationKind::Request {
            assert!(core.resolve(&mut request, ()).unwrap().is_empty());
        }
    }
    let observations = observer.observations();
    let child_end = observations
        .iter()
        .position(|o| *o == RuntimeObservation::EventEnded { context_id: child })
        .unwrap();
    let root_end = observations
        .iter()
        .position(|o| *o == RuntimeObservation::EventEnded { context_id: root })
        .unwrap();
    assert!(child_end < root_end);
}

mod teardown {
    use super::*;
    use crate::command::AbortHandle;
    use std::sync::atomic::{AtomicBool, Ordering};

    enum CancelEvent {
        Start,
        FollowUp,
        Abort,
    }

    struct CancelApp {
        dropped: Arc<AtomicBool>,
    }

    struct WorkGuard(Arc<AtomicBool>);

    impl Drop for WorkGuard {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    impl crate::App for CancelApp {
        type Event = CancelEvent;
        type Model = Option<AbortHandle>;
        type ViewModel = ();
        type Effect = Effect;

        fn update(
            &self,
            event: CancelEvent,
            model: &mut Self::Model,
        ) -> Command<Effect, CancelEvent> {
            match event {
                CancelEvent::Start => {
                    let command = Command::event(CancelEvent::FollowUp);
                    *model = Some(command.abort_handle());
                    command
                }
                CancelEvent::FollowUp => {
                    let guard = WorkGuard(self.dropped.clone());
                    Command::new(move |ctx| async move {
                        let _guard = guard;
                        ctx.request_from_shell(Op::FollowUp).await;
                    })
                }
                CancelEvent::Abort => {
                    model.take().unwrap().abort();
                    Command::done()
                }
            }
        }

        fn view(&self, _: &Self::Model) {}
    }

    #[derive(Clone)]
    struct CleanupObserver {
        observations: TestObserver,
        dropped: Arc<AtomicBool>,
        child_id: Arc<Mutex<Option<RuntimeContextId>>>,
    }

    impl CoreRuntimeObserver for CleanupObserver {
        fn event_started(
            &self,
            context_id: RuntimeContextId,
            parent_context_id: RuntimeContextId,
            name: &'static str,
        ) {
            if parent_context_id != RuntimeContextId::ROOT {
                *self.child_id.lock().unwrap() = Some(context_id);
            }
            self.observations
                .event_started(context_id, parent_context_id, name);
        }

        fn event_ended(&self, id: RuntimeContextId) {
            if Some(id) == *self.child_id.lock().unwrap() {
                assert!(
                    self.dropped.load(Ordering::SeqCst),
                    "child work must be dropped before its end is reported"
                );
            }
            self.observations.event_ended(id);
        }
    }

    #[test]
    fn descendant_work_is_dropped_before_child_then_parent_end() {
        // Exercise ancestor abort, Core teardown, abandoned Rust requests,
        // and normal resolution. All paths must clean up before reporting ends.
        for mode in ["abort", "drop core", "drop handle", "resolve"] {
            let dropped = Arc::new(AtomicBool::new(false));
            let observer = CleanupObserver {
                observations: TestObserver::default(),
                dropped: dropped.clone(),
                child_id: Arc::default(),
            };
            let core = Core::new_with(
                CancelApp {
                    dropped: dropped.clone(),
                },
                None,
            )
            .with_runtime_observer(observer.clone());
            let effects = core.process_event(CancelEvent::Start);
            assert_eq!(effects.len(), 1);
            assert!(!dropped.load(Ordering::SeqCst));
            let RuntimeObservation::EventStarted {
                context_id: root, ..
            } = observer.observations.observations()[0]
            else {
                panic!("expected root start");
            };
            let child = observer.child_id.lock().unwrap().unwrap();
            match mode {
                "abort" => {
                    assert!(core.process_event(CancelEvent::Abort).is_empty());
                    // Aborting must clean up and report termination without
                    // relying on the shell to resolve or drop its handle.
                    assert!(dropped.load(Ordering::SeqCst));
                    assert!(
                        observer
                            .observations
                            .observations()
                            .contains(&RuntimeObservation::EventEnded { context_id: child })
                    );
                    assert!(
                        observer
                            .observations
                            .observations()
                            .contains(&RuntimeObservation::EventEnded { context_id: root })
                    );
                    // A late shell response must not report another end.
                    let Effect::Op(mut request) = effects.into_iter().next().unwrap();
                    assert!(core.resolve(&mut request, ()).unwrap().is_empty());
                    drop(core);
                }
                "drop core" => {
                    drop(core);
                    drop(effects);
                }
                "drop handle" => {
                    drop(effects);
                    assert!(core.process().is_empty());
                    assert!(dropped.load(Ordering::SeqCst));
                    assert!(
                        observer
                            .observations
                            .observations()
                            .contains(&RuntimeObservation::EventEnded { context_id: child })
                    );
                    drop(core);
                }
                "resolve" => {
                    let Effect::Op(mut request) = effects.into_iter().next().unwrap();
                    assert!(core.resolve(&mut request, ()).unwrap().is_empty());
                    assert!(dropped.load(Ordering::SeqCst));
                    assert!(
                        observer
                            .observations
                            .observations()
                            .contains(&RuntimeObservation::EventEnded { context_id: child })
                    );
                    drop(core);
                }
                _ => unreachable!(),
            }
            assert!(dropped.load(Ordering::SeqCst));
            let ends: Vec<_> = observer
                .observations
                .observations()
                .iter()
                .filter_map(|o| match o {
                    RuntimeObservation::EventEnded { context_id }
                        if *context_id == root || *context_id == child =>
                    {
                        Some(*context_id)
                    }
                    _ => None,
                })
                .collect();
            if mode == "drop core" {
                // Root-hosted commands can be destroyed in storage order on
                // Core teardown; each must still report its end exactly once.
                assert_eq!(ends.len(), 2);
                assert!(ends.contains(&child));
                assert!(ends.contains(&root));
            } else {
                assert_eq!(
                    ends,
                    [child, root],
                    "mode {mode}: each event must end once, descendants first"
                );
            }
        }
    }

    #[test]
    fn unpolled_event_host_drops_work_before_reporting_end() {
        let dropped = Arc::new(AtomicBool::new(false));
        let id = RuntimeContextId(42);
        let observer = CleanupObserver {
            observations: TestObserver::default(),
            dropped: dropped.clone(),
            child_id: Arc::new(Mutex::new(Some(id))),
        };
        let guard = WorkGuard(dropped);
        let command: Command<Effect, Event> = Command::new(move |_ctx| async move {
            let _guard = guard;
            std::future::pending::<()>().await;
        });
        let parent: Command<Effect, Event> = Command::done();
        let host = command.host_observably(
            parent.context,
            id,
            crate::core::RuntimeObservationContext::new(observer.clone()),
        );
        drop(host);
        assert_eq!(
            observer.observations.observations(),
            [RuntimeObservation::EventEnded { context_id: id }]
        );
    }
}

#[test]
fn unobserved_core_hosts_follow_up_work_without_reading_effect_metadata() {
    struct UnobservedEffect(Effect);

    impl crate::Effect for UnobservedEffect {
        fn name(&self) -> &'static str {
            panic!("unobserved hosting must not read effect names");
        }

        fn kind(&self) -> crate::OperationKind {
            panic!("unobserved hosting must not classify effects");
        }
    }

    struct UnobservedApp;

    impl crate::App for UnobservedApp {
        type Event = Event;
        type Model = ();
        type ViewModel = ();
        type Effect = UnobservedEffect;

        fn update(&self, event: Event, model: &mut ()) -> Command<UnobservedEffect, Event> {
            crate::App::update(&TestApp, event, model).map_effect(UnobservedEffect)
        }

        fn view(&self, (): &()) {}
    }

    let core = Core::new_with(UnobservedApp, ());
    let mut effects = core.process_event(Event::Start);
    assert_eq!(effects.len(), 1);
    let UnobservedEffect(Effect::Op(mut request)) = effects.remove(0);
    assert!(core.resolve(&mut request, ()).unwrap().is_empty());
}
