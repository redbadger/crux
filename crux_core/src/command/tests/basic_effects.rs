use serde::{Deserialize, Serialize};

use super::super::Command;
use crate::{capability::Operation, Request};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
struct AnOperation;
#[derive(Debug, PartialEq, Deserialize)]
struct AnOperationOutput;

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
    Start,
    Completed(AnOperationOutput),
}

// Commands can be constructed without async and dispatch basic
// effects, which are executed lazily when the command is asked for
// emitted events or effects

#[test]
fn done_can_be_created() {
    let mut cmd: Command<Effect, Event> = Command::done();

    assert!(cmd.is_done())
}

#[test]
fn notify_can_be_created_with_an_operation() {
    let mut cmd: Command<Effect, Event> = Command::notify_shell(AnOperation).into();

    assert!(!cmd.is_done());

    assert!(cmd.effects().next().is_some());

    assert!(cmd.is_done());
}

#[test]
fn notify_effect_can_be_inspected() {
    let mut cmd: Command<Effect, Event> = Command::notify_shell(AnOperation).into();

    let effects = cmd.effects().next();

    assert!(effects.is_some());

    let Effect::AnEffect(request) = effects.unwrap();

    assert_eq!(request.operation, AnOperation)
}

#[test]
fn request_effect_can_be_inspected() {
    let mut cmd = Command::request_from_shell(AnOperation).then_send(Event::Completed);

    let effect = cmd.effects().next();
    assert!(effect.is_some());

    let Effect::AnEffect(request) = effect.unwrap();

    assert_eq!(request.operation, AnOperation)
}

// ANCHOR: basic_test
#[test]
fn request_effect_can_be_resolved() {
    let mut cmd = Command::request_from_shell(AnOperation).then_send(Event::Completed);

    let effect = cmd.effects().next();
    assert!(effect.is_some());

    let Effect::AnEffect(mut request) = effect.unwrap();

    assert_eq!(request.operation, AnOperation);

    request
        .resolve(AnOperationOutput)
        .expect("Resolve should succeed");

    let event = cmd.events().next().unwrap();

    assert_eq!(event, Event::Completed(AnOperationOutput));

    assert!(cmd.is_done())
}
// ANCHOR_END: basic_test

#[test]
fn stream_effect_can_be_resolved_multiple_times() {
    let mut cmd = Command::stream_from_shell(AnOperation).then_send(Event::Completed);

    let effect = cmd.effects().next();

    assert!(cmd.events().next().is_none());

    let Effect::AnEffect(mut request) = effect.unwrap();

    assert_eq!(request.operation, AnOperation);

    request
        .resolve(AnOperationOutput)
        .expect("Resolve should succeed");

    let event = cmd.events().next().unwrap();

    assert!(matches!(event, Event::Completed(AnOperationOutput)));

    assert!(cmd.effects().next().is_none());
    assert!(cmd.events().next().is_none());
    assert!(!cmd.is_done());

    request
        .resolve(AnOperationOutput)
        .expect("Resolve should succeed");

    let event = cmd.events().next().unwrap();

    assert!(matches!(event, Event::Completed(AnOperationOutput)));
}

#[test]
fn event_can_be_created() {
    let mut cmd: Command<Effect, _> = Command::event(Event::Start);

    let event = cmd.events().next().unwrap();

    assert_eq!(event, Event::Start);
}
