use futures::StreamExt;
use serde::Serialize;

use crate::{capability::Operation, Request};

use super::super::Command;

#[derive(Debug, Clone, PartialEq, Serialize)]
enum Op {
    Basic,
    Abort,
}

impl Operation for Op {
    type Output = usize;
}

enum Effect {
    Op(Request<Op>),
}

impl From<Request<Op>> for Effect {
    fn from(value: Request<Op>) -> Self {
        Effect::Op(value)
    }
}

#[derive(Debug, PartialEq)]
enum Event {
    OpDone(usize),
    Ping,
}

#[test]
fn spawn_returns_join_handle() {
    let mut cmd = Command::new(|ctx| async move {
        let task_join = ctx.spawn({
            let ctx = ctx.clone();
            async move {
                ctx.request_from_shell(Op::Basic).await;
            }
        });

        task_join.await;

        ctx.send_event(Event::Ping);
    });

    assert!(cmd.events().next().is_none());

    let effect = cmd.effects().next().unwrap();
    let Effect::Op(mut request) = effect;

    request.resolve(1).expect("should resolve");

    let event = cmd.events().next().unwrap();

    assert_eq!(event, Event::Ping);

    assert!(cmd.is_done());
}

#[test]
fn join_handle_can_abort_a_task() {
    let mut cmd = Command::new(|ctx| async move {
        let stream_handle = ctx.spawn({
            let ctx = ctx.clone();

            async move {
                let mut stream = ctx.stream_from_shell(Op::Basic);

                while stream.next().await.is_some() {
                    ctx.send_event(Event::Ping);
                }
            }
        });

        ctx.spawn({
            let ctx = ctx.clone();
            async move {
                ctx.request_from_shell(Op::Abort).await;

                stream_handle.abort();
            }
        });
    });

    assert!(cmd.events().next().is_none());

    let mut effects: Vec<_> = cmd.effects().collect();

    let Effect::Op(mut stream_request) = effects.remove(0);
    let Effect::Op(mut abort_request) = effects.remove(0);

    assert_eq!(abort_request.operation, Op::Abort);
    assert_eq!(stream_request.operation, Op::Basic);

    for i in 1..10 {
        stream_request.resolve(i).expect("to resolve");
        let event = cmd.events().next().unwrap();

        assert_eq!(event, Event::Ping);
    }

    assert!(!cmd.is_done());

    abort_request.resolve(0).expect("to resolve");

    // Stream has ended
    stream_request.resolve(1).expect("to resolve"); // FIXME: should this be an error?
    assert!(cmd.events().next().is_none());

    // so has the whole command
    assert!(cmd.is_done());
}

#[test]
fn tasks_can_be_aborted_immediately() {
    let mut cmd: Command<Effect, Event> = Command::new(|ctx| async move {
        let handle = ctx.spawn({
            let ctx = ctx.clone();
            async move {
                ctx.request_from_shell(Op::Abort).await;
            }
        });

        handle.abort();
    });

    // Need to poll at least once
    assert!(cmd.effects().next().is_none());
    assert!(cmd.events().next().is_none());

    // so has the whole command
    assert!(cmd.is_done());
}

#[test]
fn aborted_tasks_notify_their_join_handles() {
    let mut cmd = Command::new(|ctx| async move {
        let stream_handle = ctx.spawn({
            let ctx = ctx.clone();

            async move {
                let mut stream = ctx.stream_from_shell(Op::Basic);

                while stream.next().await.is_some() {
                    ctx.send_event(Event::Ping);
                }
            }
        });

        ctx.spawn({
            let ctx = ctx.clone();
            let stream_handle = stream_handle.clone();
            async move {
                ctx.request_from_shell(Op::Abort).await;

                stream_handle.abort();
            }
        });

        ctx.spawn({
            let ctx = ctx.clone();
            async move {
                stream_handle.await;

                ctx.send_event(Event::OpDone(3));
            }
        });
    });

    assert!(cmd.events().next().is_none());

    let mut effects: Vec<_> = cmd.effects().collect();

    let Effect::Op(mut stream_request) = effects.remove(0);
    let Effect::Op(mut abort_request) = effects.remove(0);

    assert_eq!(abort_request.operation, Op::Abort);
    assert_eq!(stream_request.operation, Op::Basic);

    for i in 1..10 {
        stream_request.resolve(i).expect("to resolve");
        let event = cmd.events().next().unwrap();

        assert_eq!(event, Event::Ping);
    }

    assert!(!cmd.is_done());

    abort_request.resolve(0).expect("to resolve");

    // Stream has ended
    stream_request.resolve(1).expect("to resolve"); // FIXME: should this be an error?

    // Third task woke and produced an event
    assert_eq!(Event::OpDone(3), cmd.events().next().unwrap());

    // Command has completed
    assert!(cmd.is_done());
}

#[test]
fn commands_can_be_aborted() {
    let mut cmd: Command<Effect, Event> = Command::all([
        Command::request_from_shell(Op::Basic).then_send(Event::OpDone),
        Command::request_from_shell(Op::Basic).then_send(Event::OpDone),
    ]);

    let handle = cmd.abort_handle();

    assert!(!cmd.was_aborted());

    let mut effects: Vec<_> = cmd.effects().collect();
    assert_eq!(effects.len(), 2);

    handle.abort();

    // Command is now finished
    assert!(cmd.is_done());
    assert!(cmd.was_aborted());

    // We can still resolve requests, but nothing happens

    let Effect::Op(mut first_request) = effects.remove(0);
    let Effect::Op(mut second_request) = effects.remove(0);

    first_request.resolve(1).expect("to resolve");
    second_request.resolve(2).expect("to resolve");

    assert!(cmd.events().next().is_none());
    assert!(cmd.effects().next().is_none());
}
