//! Pending effects are filtered by Core, never by command composition.
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::task::Poll;

use crux_core::{App, Command, Core, capability::Operation, command::AbortHandle};
use futures::{FutureExt as _, StreamExt as _, future::poll_fn};

struct Op;
impl Operation for Op {
    type Output = ();
}

#[crux_core::macros::effect]
enum Effect {
    Op(Op),
}

enum Event {
    Abandon { stream: bool, poll_once: bool },
    QueueThenAbort,
    Abort,
}

#[derive(Default)]
struct TestApp {
    forwarded: Arc<AtomicUsize>,
}

impl App for TestApp {
    type Event = Event;
    type Effect = Effect;
    type Model = Option<AbortHandle>;
    type ViewModel = ();

    fn update(&self, event: Event, model: &mut Self::Model) -> Command<Effect, Event> {
        let command = match event {
            Event::Abandon { stream, poll_once } => Command::new(move |ctx| async move {
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
            }),
            Event::QueueThenAbort => Command::new(|ctx| async move {
                ctx.send_event(Event::Abort);
                ctx.request_from_shell(Op).await;
            }),
            Event::Abort => {
                model.take().unwrap().abort();
                return Command::done();
            }
        };
        let forwarded = self.forwarded.clone();
        let command = command.map_effect(move |effect| {
            forwarded.fetch_add(1, Ordering::Relaxed);
            effect
        });
        *model = Some(command.abort_handle());
        command
    }

    fn view(&self, _: &Self::Model) {}
}

#[test]
fn abandoned_waits_pass_through_composition_but_are_not_dispatched() {
    for stream in [false, true] {
        for poll_once in [false, true] {
            let app = TestApp::default();
            let forwarded = app.forwarded.clone();
            let core = Core::new_with(app, None);
            let effects = core.process_event(Event::Abandon { stream, poll_once });
            assert_eq!(
                forwarded.load(Ordering::Relaxed),
                1 + usize::from(poll_once)
            );
            assert_eq!(effects.len(), 1);
            let Effect::Op(request) = &effects[0];
            assert_eq!(request.handle.kind(), crux_core::OperationKind::Notify);
        }
    }
}

#[test]
fn abort_before_core_dispatch_discards_an_already_queued_effect() {
    let app = TestApp::default();
    let forwarded = app.forwarded.clone();
    let core = Core::new_with(app, None);
    assert!(core.process_event(Event::QueueThenAbort).is_empty());
    assert_eq!(forwarded.load(Ordering::Relaxed), 1);
}
