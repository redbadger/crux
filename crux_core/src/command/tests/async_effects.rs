use futures::{join, select, FutureExt};
use serde::{Deserialize, Serialize};

use super::super::Command;
use crate::{capability::Operation, Request};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
enum AnOperation {
    One,
    Two,
    Three,
}
#[derive(Debug, PartialEq, Deserialize)]
enum AnOperationOutput {
    One,
    Two,
    Three,
}

impl Operation for AnOperation {
    type Output = AnOperationOutput;
}

enum Effect {
    AnEffect(Request<AnOperation>),
}

impl From<Request<AnOperation>> for Effect {
    fn from(request: Request<AnOperation>) -> Self {
        Self::AnEffect(request)
    }
}

#[derive(Debug, PartialEq)]
enum Event {
    Completed(AnOperationOutput),
    Aborted,
}

// Beyond the basic constructors, Command::new can be called directly
// and async code can be used to orchestrate effects. This is just async rust
// but we're checking the Command's executor works correctly

#[test]
fn effects_execute_in_sequence() {
    let mut cmd = Command::new(|ctx| async move {
        let output = ctx.request_from_shell(AnOperation::One).await;
        ctx.send_event(Event::Completed(output));
        let output = ctx.request_from_shell(AnOperation::Two).await;
        ctx.send_event(Event::Completed(output));
    });

    assert!(cmd.events().next().is_none());

    let effect = cmd.effects().next().unwrap();
    let Effect::AnEffect(mut request) = effect;

    assert_eq!(request.operation, AnOperation::One);

    request
        .resolve(AnOperationOutput::One)
        .expect("request should resolve");

    let event = cmd.events().next().unwrap();

    assert_eq!(event, Event::Completed(AnOperationOutput::One));

    assert!(cmd.events().next().is_none());

    let effect = cmd.effects().next().unwrap();
    let Effect::AnEffect(mut request) = effect;

    assert_eq!(request.operation, AnOperation::Two);

    request
        .resolve(AnOperationOutput::Two)
        .expect("request should resolve");

    assert!(cmd.effects().next().is_none());

    let event = cmd.events().next().unwrap();

    assert_eq!(event, Event::Completed(AnOperationOutput::Two))
}

#[test]
fn effects_execute_in_parallel() {
    let mut cmd = Command::new(|ctx| async move {
        let (first, second) = join!(
            ctx.request_from_shell(AnOperation::One),
            ctx.request_from_shell(AnOperation::Two)
        );

        ctx.send_event(Event::Completed(first));
        ctx.send_event(Event::Completed(second));
    });

    assert!(cmd.events().next().is_none());

    let mut effects: Vec<_> = cmd.effects().collect();
    let Effect::AnEffect(mut first_request) = effects.remove(0);
    let Effect::AnEffect(mut second_request) = effects.remove(0);

    assert_eq!(first_request.operation, AnOperation::One);
    assert_eq!(second_request.operation, AnOperation::Two);

    first_request
        .resolve(AnOperationOutput::One)
        .expect("request should resolve");

    assert!(cmd.events().next().is_none());

    second_request
        .resolve(AnOperationOutput::Two)
        .expect("request should resolve");

    assert!(cmd.effects().next().is_none());

    let mut events: Vec<_> = cmd.events().collect();

    assert_eq!(events.len(), 2);

    assert_eq!(events.remove(0), Event::Completed(AnOperationOutput::One));
    assert_eq!(events.remove(0), Event::Completed(AnOperationOutput::Two));
}

#[test]
fn effects_race() {
    let mut cmd = Command::new(|ctx| async move {
        select! {
            output = ctx.request_from_shell(AnOperation::One).fuse() => ctx.send_event(Event::Completed(output)),
            output = ctx.request_from_shell(AnOperation::Two).fuse() => ctx.send_event(Event::Completed(output)),
            output = ctx.request_from_shell(AnOperation::Three).fuse() => ctx.send_event(Event::Completed(output))
        };
    });

    assert!(cmd.events().next().is_none());

    let mut effects: Vec<_> = cmd.effects().collect();
    let Effect::AnEffect(mut third_request) = effects.remove(0);
    let Effect::AnEffect(mut second_request) = effects.remove(0);
    let Effect::AnEffect(mut first_request) = effects.remove(0);

    assert_eq!(first_request.operation, AnOperation::One);
    assert_eq!(second_request.operation, AnOperation::Two);
    assert_eq!(third_request.operation, AnOperation::Three);

    second_request
        .resolve(AnOperationOutput::Two)
        .expect("request should resolve");

    first_request
        .resolve(AnOperationOutput::One)
        .expect("request should resolve");

    let mut events: Vec<_> = cmd.events().collect();

    assert_eq!(events.len(), 1);
    assert_eq!(events.remove(0), Event::Completed(AnOperationOutput::Two));

    third_request
        .resolve(AnOperationOutput::Three)
        .expect("request should resolve");

    // The select! has finished
    assert!(cmd.events().next().is_none())
}

#[test]
fn effects_can_spawn_communicating_tasks() {
    // We make two tasks which communicate over a channel
    // the first sends effect requests and forwards outputs to the second
    // the second sends them back out wrapped in events
    // the first task continues until an ::Abort resolution is sent
    // the second continues until it can't read from the channel

    let mut cmd = Command::new(|ctx| async move {
        let (tx, rx) = async_channel::unbounded();

        ctx.spawn(|ctx| async move {
            loop {
                let output = ctx.request_from_shell(AnOperation::One).await;

                tx.send(output).await.unwrap();
            }
        });

        ctx.spawn(|ctx| async move {
            while let Ok(value) = rx.recv().await {
                ctx.send_event(Event::Completed(value));
            }

            ctx.send_event(Event::Aborted);
        });
    });

    let mut effects: Vec<_> = cmd.effects().collect();

    assert_eq!(effects.len(), 1);

    let Effect::AnEffect(mut first_request) = effects.remove(0);
    first_request
        .resolve(AnOperationOutput::One)
        .expect("request should resolve");

    let mut effects: Vec<_> = cmd.effects().collect();
    let events: Vec<_> = cmd.events().collect();

    assert_eq!(effects.len(), 1);
    assert_eq!(events.len(), 1);

    assert_eq!(events[0], Event::Completed(AnOperationOutput::One));

    let Effect::AnEffect(mut first_request) = effects.remove(0);
    first_request
        .resolve(AnOperationOutput::Two)
        .expect("request should resolve");

    let mut effects: Vec<_> = cmd.effects().collect();
    let events: Vec<_> = cmd.events().collect();

    assert_eq!(effects.len(), 1);
    assert_eq!(events.len(), 1);

    assert_eq!(events[0], Event::Completed(AnOperationOutput::Two));

    let Effect::AnEffect(first_request) = effects.remove(0);

    // Dropping the task cancels it
    drop(first_request);

    assert!(cmd.effects().next().is_none());

    assert_eq!(cmd.events().next().unwrap(), Event::Aborted);

    assert!(cmd.is_done());
}

#[test]
fn tasks_can_be_spawned_on_existing_effects() {
    let mut cmd: Command<Effect, Event> = Command::done();

    assert!(cmd.is_done());
    assert!(cmd.effects().next().is_none());

    cmd.spawn(|ctx| async move {
        ctx.request_from_shell(AnOperation::One).await;
    });

    // Command is not done any more
    assert!(!cmd.is_done());
    assert!(cmd.effects().next().is_some());
}
