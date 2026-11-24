use futures::channel::oneshot;

use crate::{Command, Request, capability::Operation};

struct Work;

impl Operation for Work {
    type Output = ();
}

// The main task finishes immediately, leaving an explicitly spawned task alive.
fn owned_work() -> Command<Request<Work>, u8> {
    Command::new(|ctx| {
        ctx.spawn(|ctx| async move {
            ctx.request_from_shell(Work).await;
            ctx.send_event(1);
        });
        async {}
    })
}

#[test]
fn map_event_preserves_waiting_inside_and_outside_the_map() {
    let mut command = owned_work()
        .then(Command::event(2))
        .map_event(Box::new)
        .then(Command::event(Box::new(3)));

    assert!(command.events().next().is_none());
    assert!(!command.is_done());
    let mut request = command.effects().next().expect("owned task must run");
    request.resolve(()).unwrap();
    assert_eq!(
        command.events().map(|event| *event).collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert!(command.is_done());
}

#[test]
fn map_effect_preserves_waiting_inside_and_outside_the_map() {
    let mut command = owned_work()
        .then(Command::event(2))
        .map_effect(Box::new)
        .then(Command::event(3));

    assert!(command.events().next().is_none());
    assert!(!command.is_done());
    let mut request = command.effects().next().expect("owned task must run");
    request.resolve(()).unwrap();
    assert_eq!(command.events().collect::<Vec<_>>(), [1, 2, 3]);
    assert!(command.is_done());
}

fn check_mapped_cancellation<E, V>(
    map: impl FnOnce(Command<Request<Work>, u8>) -> Command<E, V>,
    poll_first: bool,
    abort_inner: bool,
) where
    E: Send + Unpin + 'static,
    V: Send + Unpin + 'static,
{
    let (sender, mut cleanup) = oneshot::channel::<()>();
    let child = Command::new(move |ctx| async move {
        let _sender = sender;
        ctx.request_from_shell(Work).await;
        ctx.send_event(1);
    });
    let inner = Command::all([child]);
    let inner_abort = inner.abort_handle();
    let mut mapped = map(inner);
    let abort = if abort_inner {
        inner_abort
    } else {
        mapped.abort_handle()
    };

    // Retain any shell handle: cancellation must drop the task independently.
    let effects = if poll_first {
        let effects = mapped.effects().collect::<Vec<_>>();
        assert_eq!(effects.len(), 1);
        assert!(!mapped.is_done());
        effects
    } else {
        Vec::new()
    };
    assert_eq!(cleanup.try_recv().unwrap(), None);
    abort.abort();
    assert!(mapped.is_done());
    assert!(
        cleanup.try_recv().is_err(),
        "owned child must be dropped before completion"
    );
    assert!(mapped.events().next().is_none());
    assert!(mapped.effects().next().is_none());
    drop(effects);
}

#[test]
fn mapping_preserves_inner_and_outer_abort_before_polling_and_while_suspended() {
    for poll_first in [false, true] {
        for abort_inner in [false, true] {
            check_mapped_cancellation(
                |command| command.map_event(Box::new),
                poll_first,
                abort_inner,
            );
            check_mapped_cancellation(
                |command| command.map_effect(Box::new),
                poll_first,
                abort_inner,
            );
            check_mapped_cancellation(
                |command| command.map_event(Box::new).map_effect(Box::new),
                poll_first,
                abort_inner,
            );
        }
    }
}

#[test]
fn aborting_a_mapped_subcommand_releases_then_after_cleanup() {
    let (sender, mut cleanup) = oneshot::channel::<()>();
    let child: Command<Request<Work>, u8> = Command::new(move |ctx| async move {
        let _sender = sender;
        ctx.request_from_shell(Work).await;
    });
    let abort = child.abort_handle();
    let mut command = child.map_event(Box::new).then(Command::event(Box::new(2)));
    let request = command.effects().next().unwrap();
    assert!(command.events().next().is_none());
    abort.abort();
    assert_eq!(
        command.events().map(|event| *event).collect::<Vec<_>>(),
        [2]
    );
    assert!(cleanup.try_recv().is_err());
    assert!(command.is_done());
    drop(request);
}
