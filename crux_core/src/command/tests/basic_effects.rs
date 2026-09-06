use serde::{Deserialize, Serialize};

use super::super::Command;
use crate::{Request, capability::Operation};

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

    assert!(cmd.is_done());
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

    assert_eq!(request.operation, AnOperation);
}

#[test]
fn request_effect_can_be_inspected() {
    let mut cmd = Command::request_from_shell(AnOperation).then_send(Event::Completed);

    let effect = cmd.effects().next();
    assert!(effect.is_some());

    let Effect::AnEffect(request) = effect.unwrap();

    assert_eq!(request.operation, AnOperation);
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

    assert!(cmd.is_done());
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

/// Operations which declare a [`RequestKind`] statically, sent through the
/// constructor that matches. The mismatched combinations are compile errors,
/// covered by the `trybuild` suite in `crux_core/tests/ui`.
mod typed_operations {
    use super::super::super::Command;
    use crate::{Request, RequestKind, capability::Operation, operation};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
    struct ANotification;

    impl Operation for ANotification {
        type Output = ();
        const KIND: Option<RequestKind> = Some(RequestKind::Notify);
    }

    impl operation::Notify for ANotification {}

    #[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
    struct ARequest;

    impl Operation for ARequest {
        type Output = usize;
        const KIND: Option<RequestKind> = Some(RequestKind::Request);
    }

    impl operation::Request for ARequest {}

    #[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
    struct AStream;

    impl Operation for AStream {
        type Output = usize;
        const KIND: Option<RequestKind> = Some(RequestKind::Stream);
    }

    impl operation::Stream for AStream {}

    enum Effect {
        Notification(Request<ANotification>),
        OneShot(Request<ARequest>),
        Streaming(Request<AStream>),
    }

    impl From<Request<ANotification>> for Effect {
        fn from(request: Request<ANotification>) -> Self {
            Self::Notification(request)
        }
    }

    impl From<Request<ARequest>> for Effect {
        fn from(request: Request<ARequest>) -> Self {
            Self::OneShot(request)
        }
    }

    impl From<Request<AStream>> for Effect {
        fn from(request: Request<AStream>) -> Self {
            Self::Streaming(request)
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    enum Event {
        Completed(usize),
    }

    #[test]
    fn a_notify_operation_can_be_notified() {
        let mut cmd: Command<Effect, Event> = Command::notify_shell(ANotification).into();

        let effect = cmd.effects().next().expect("an effect");
        let Effect::Notification(request) = effect else {
            panic!("expected a notification");
        };

        assert_eq!(request.operation, ANotification);
        assert!(cmd.is_done());
    }

    #[test]
    fn a_request_operation_can_be_requested() {
        let mut cmd: Command<Effect, Event> =
            Command::request_from_shell(ARequest).then_send(Event::Completed);

        let effect = cmd.effects().next().expect("an effect");
        let Effect::OneShot(mut request) = effect else {
            panic!("expected a one-shot request");
        };

        assert_eq!(request.operation, ARequest);

        request.resolve(1).expect("resolve should succeed");

        assert_eq!(cmd.events().next(), Some(Event::Completed(1)));
        assert!(cmd.is_done());
    }

    #[test]
    fn a_stream_operation_can_be_streamed() {
        let mut cmd: Command<Effect, Event> =
            Command::stream_from_shell(AStream).then_send(Event::Completed);

        let effect = cmd.effects().next().expect("an effect");
        let Effect::Streaming(mut request) = effect else {
            panic!("expected a stream request");
        };

        assert_eq!(request.operation, AStream);

        request.resolve(1).expect("resolve should succeed");
        assert_eq!(cmd.events().next(), Some(Event::Completed(1)));

        request.resolve(2).expect("resolve should succeed");
        assert_eq!(cmd.events().next(), Some(Event::Completed(2)));

        assert!(!cmd.is_done());
    }
}

/// An operation which declares no kind is accepted by all three constructors —
/// the behaviour every `Operation` implementation had before `KIND` existed.
#[test]
fn an_operation_without_a_kind_takes_any_constructor() {
    assert_eq!(AnOperation::KIND, None);

    let mut notified: Command<Effect, Event> = Command::notify_shell(AnOperation).into();
    let mut requested: Command<Effect, Event> =
        Command::request_from_shell(AnOperation).then_send(Event::Completed);
    let mut streamed: Command<Effect, Event> =
        Command::stream_from_shell(AnOperation).then_send(Event::Completed);

    for cmd in [&mut notified, &mut requested, &mut streamed] {
        assert!(cmd.effects().next().is_some());
    }
}
