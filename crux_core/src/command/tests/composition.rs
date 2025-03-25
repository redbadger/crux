use serde::{Deserialize, Serialize};

use crate::{capability::Operation, Command, Request};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
struct ToString(usize);

impl Operation for ToString {
    type Output = String;
}

#[derive(PartialEq, Clone, Serialize, Deserialize)]
struct HowMany;

impl Operation for HowMany {
    type Output = usize;
}

enum Effect {
    Convert(Request<ToString>),
}

impl From<Request<ToString>> for Effect {
    fn from(value: Request<ToString>) -> Self {
        Effect::Convert(value)
    }
}

#[allow(dead_code)]
enum ParentEffect {
    Convert(Request<ToString>),
    Count(Request<HowMany>),
}

impl From<Request<ToString>> for ParentEffect {
    fn from(value: Request<ToString>) -> Self {
        ParentEffect::Convert(value)
    }
}

#[derive(Debug, PartialEq)]
enum Event {
    Converted(String),
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
enum ParentEvent {
    Converted(String),
    Counted(usize),
}

#[test]
fn map_effect() {
    let cmd: Command<Effect, Event> =
        Command::request_from_shell(ToString(3)).then_send(Event::Converted);

    let mut mapped_cmd = cmd.map_effect(|ef| match ef {
        Effect::Convert(request) => ParentEffect::Convert(request),
    });

    let effect = mapped_cmd.effects().next().unwrap();

    let ParentEffect::Convert(mut request) = effect else {
        panic!("Wrong effect variant!");
    };

    assert_eq!(request.operation, ToString(3));

    request
        .resolve("three".to_string())
        .expect("should resolve");

    let event = mapped_cmd.events().next().unwrap();

    assert_eq!(event, Event::Converted("three".to_string()));
}

#[test]
fn map_event() {
    let cmd: Command<Effect, Event> =
        Command::request_from_shell(ToString(3)).then_send(Event::Converted);

    let mut mapped_cmd = cmd.map_event(|ef| match ef {
        Event::Converted(out) => ParentEvent::Converted(out),
    });

    let effect = mapped_cmd.effects().next().unwrap();

    let Effect::Convert(mut request) = effect;

    assert_eq!(request.operation, ToString(3));

    request
        .resolve("three".to_string())
        .expect("should resolve");

    let event = mapped_cmd.events().next().unwrap();

    assert_eq!(event, ParentEvent::Converted("three".to_string()));
}
