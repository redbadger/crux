use std::{future::Future, pin::Pin, task::Poll};

use serde::{Deserialize, Serialize};

use crate::{capability::Operation, Command, Request};

struct ImmediateWake {
    wake_count: usize,
}

impl Future for ImmediateWake {
    type Output = usize;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if self.wake_count < 1 {
            Poll::Ready(self.wake_count)
        } else {
            self.wake_count -= 1;

            cx.waker().wake_by_ref();

            Poll::Pending
        }
    }
}

#[derive(PartialEq, Debug)]
enum Event {
    Count(usize),
}

#[derive(PartialEq, Clone, Serialize, Deserialize)]
struct Op;

impl Operation for Op {
    type Output = usize;
}

enum Effect {
    Effect(Request<Op>),
}

impl From<Request<Op>> for Effect {
    fn from(value: Request<Op>) -> Self {
        Effect::Effect(value)
    }
}

#[test]
fn tasks_with_no_more_wakers_are_evicted() {
    let mut cmd = Command::request_from_shell(Op).then_send(Event::Count);

    let effect = cmd.effects().next().expect("should see effect");

    let Effect::Effect(request) = effect;

    assert!(!cmd.is_done());

    drop(request);

    assert!(cmd.is_done());
}

#[test]
fn tasks_which_wake_themselves_and_drop_wakers_are_not_evicted() {
    let mut cmd = Command::<Effect, Event>::new(|ctx| async move {
        let count = ImmediateWake { wake_count: 2 }.await;
        ctx.send_event(Event::Count(count));
    });

    assert_eq!(cmd.events().next(), Some(Event::Count(0)));

    assert!(cmd.is_done());
}
