use serde::{Deserialize, Serialize};

use super::super::Command;
use crate::{capability::Operation, Request};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
enum AnOperation {
    One,
    Two,
    More([u8; 2]),
}

#[derive(Debug, PartialEq, Deserialize)]
enum AnOperationOutput {
    One,
    Two,
    Other([u8; 2]),
}

impl Operation for AnOperation {
    type Output = AnOperationOutput;
}

#[derive(Debug)]
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
}

#[test]
fn then() {
    let cmd_one = Command::request_from_shell(AnOperation::One).then_send(Event::Completed);
    let cmd_two = Command::request_from_shell(AnOperation::Two).then_send(Event::Completed);

    let mut cmd = cmd_one.then(cmd_two);

    assert!(cmd.events().next().is_none());

    let effect = cmd.effects().next().unwrap();
    let Effect::AnEffect(mut request) = effect;

    assert_eq!(request.operation, AnOperation::One);

    request
        .resolve(AnOperationOutput::One)
        .expect("request should resolve");

    let event = cmd.events().next().unwrap();

    assert_eq!(event, Event::Completed(AnOperationOutput::One));

    let effect = cmd.effects().next().unwrap();
    let Effect::AnEffect(mut request) = effect;

    assert_eq!(request.operation, AnOperation::Two);

    request
        .resolve(AnOperationOutput::Two)
        .expect("request should resolve");

    assert!(cmd.effects().next().is_none());

    let event = cmd.events().next().unwrap();

    assert_eq!(event, Event::Completed(AnOperationOutput::Two));

    assert!(cmd.is_done());
}

#[test]
fn chaining() {
    let mut cmd = Command::request_from_shell(AnOperation::More([3, 4]))
        .then_request(|first| {
            let AnOperationOutput::Other(first) = first else {
                // TODO: how do I bail quietly here?
                panic!("Invalid output!")
            };

            let second = [first[0] + 1, first[1] + 1];

            Command::request_from_shell(AnOperation::More(second))
        })
        .then_send(Event::Completed);

    let effect = cmd.effects().next().unwrap();
    assert!(cmd.events().next().is_none());

    let Effect::AnEffect(mut request) = effect;

    assert_eq!(request.operation, AnOperation::More([3, 4]));
    request
        .resolve(AnOperationOutput::Other([1, 2]))
        .expect("to resolve");

    let effect = cmd.effects().next().unwrap();
    assert!(cmd.events().next().is_none());

    let Effect::AnEffect(mut request) = effect;
    assert_eq!(request.operation, AnOperation::More([2, 3]));

    request
        .resolve(AnOperationOutput::Other([1, 2]))
        .expect("to resolve");

    let event = cmd.events().next().unwrap();
    assert!(cmd.effects().next().is_none());

    assert_eq!(event, Event::Completed(AnOperationOutput::Other([1, 2])));

    assert!(cmd.is_done());
}

#[test]
fn long_chain_support() {
    let mut cmd = Command::request_from_shell(AnOperation::More([3, 4]))
        .then_request(|first| {
            let AnOperationOutput::Other(first) = first else {
                // TODO: how do I bail quietly here?
                panic!("Invalid output!")
            };

            let second = [first[0] + 1, first[1] + 1];

            Command::request_from_shell(AnOperation::More(second))
        })
        .then_request(|second| {
            let AnOperationOutput::Other(second) = second else {
                // TODO: how do I bail quietly here?
                panic!("Invalid output!")
            };

            let second = [second[0] + 2, second[1] + 2];

            Command::request_from_shell(AnOperation::More(second))
        })
        .then_send(Event::Completed);

    let effect = cmd.effects().next().unwrap();
    assert!(cmd.events().next().is_none());

    let Effect::AnEffect(mut request) = effect;

    assert_eq!(request.operation, AnOperation::More([3, 4]));
    request
        .resolve(AnOperationOutput::Other([1, 2]))
        .expect("to resolve");

    let effect = cmd.effects().next().unwrap();
    assert!(cmd.events().next().is_none());

    let Effect::AnEffect(mut request) = effect;
    assert_eq!(request.operation, AnOperation::More([2, 3]));

    request
        .resolve(AnOperationOutput::Other([2, 3]))
        .expect("to resolve");

    let effect = cmd.effects().next().unwrap();
    assert!(cmd.events().next().is_none());

    let Effect::AnEffect(mut request) = effect;
    assert_eq!(request.operation, AnOperation::More([4, 5]));

    request
        .resolve(AnOperationOutput::Other([4, 5]))
        .expect("to resolve");

    let event = cmd.events().next().unwrap();
    assert!(cmd.effects().next().is_none());

    assert_eq!(event, Event::Completed(AnOperationOutput::Other([4, 5])));

    assert!(cmd.is_done());
}

#[test]
fn and() {
    let cmd_one = Command::request_from_shell(AnOperation::One).then_send(Event::Completed);
    let cmd_two = Command::request_from_shell(AnOperation::Two).then_send(Event::Completed);

    let mut cmd = cmd_one.and(cmd_two);

    assert!(cmd.events().next().is_none());

    let mut effects: Vec<_> = cmd.effects().collect();

    assert_eq!(effects.len(), 2);

    let Effect::AnEffect(mut request) = effects.remove(0);

    assert_eq!(request.operation, AnOperation::One);

    request
        .resolve(AnOperationOutput::One)
        .expect("request should resolve");

    // Still the original effects
    let Effect::AnEffect(mut request) = effects.remove(0);

    assert_eq!(request.operation, AnOperation::Two);

    request
        .resolve(AnOperationOutput::Two)
        .expect("request should resolve");

    assert!(cmd.effects().next().is_none());

    let events: Vec<_> = cmd.events().collect();

    assert_eq!(events[0], Event::Completed(AnOperationOutput::One));
    assert_eq!(events[1], Event::Completed(AnOperationOutput::Two));

    eprintln!("! Running cmd.is_done()");
    assert!(cmd.is_done());
}

#[test]
fn and_doesnt_blow_the_stack() {
    let mut cmd: Command<Effect, Event> = Command::done();

    for _ in 1..2000 {
        cmd = cmd.and(Command::done());
    }

    // Polling the task should work
    let _ = cmd.effects();
}

#[test]
fn all_doesnt_blow_the_stack() {
    let commands: Vec<Command<Effect, Event>> = (1..2000).map(|_| Command::done()).collect();
    let mut cmd = Command::all(commands);

    // Polling the task should work
    let _ = cmd.effects();
}

#[test]
fn all() {
    let cmd_one = Command::request_from_shell(AnOperation::One).then_send(Event::Completed);
    let cmd_two = Command::request_from_shell(AnOperation::Two).then_send(Event::Completed);
    let cmd_three = Command::request_from_shell(AnOperation::One).then_send(Event::Completed);

    let mut cmd = Command::all([cmd_one, cmd_two, cmd_three]);

    assert!(cmd.events().next().is_none());

    let mut effects: Vec<_> = cmd.effects().collect();

    assert_eq!(effects.len(), 3);

    let Effect::AnEffect(mut request) = effects.remove(0);

    assert_eq!(request.operation, AnOperation::One);

    request
        .resolve(AnOperationOutput::One)
        .expect("request should resolve");

    // Still the original effects
    let Effect::AnEffect(mut request) = effects.remove(0);

    assert_eq!(request.operation, AnOperation::Two);

    request
        .resolve(AnOperationOutput::Two)
        .expect("request should resolve");

    assert!(cmd.effects().next().is_none());

    // Still the original effects
    let Effect::AnEffect(mut request) = effects.remove(0);

    assert_eq!(request.operation, AnOperation::One);

    request
        .resolve(AnOperationOutput::Two)
        .expect("request should resolve");

    assert!(cmd.effects().next().is_none());

    let events: Vec<_> = cmd.events().collect();

    assert_eq!(events[0], Event::Completed(AnOperationOutput::One));
    assert_eq!(events[1], Event::Completed(AnOperationOutput::Two));
    assert_eq!(events[1], Event::Completed(AnOperationOutput::Two));

    assert!(cmd.is_done());
}

#[test]
fn iterator_of_commands_collects_as_all() {
    let cmd_one = Command::request_from_shell(AnOperation::One).then_send(Event::Completed);
    let cmd_two = Command::request_from_shell(AnOperation::Two).then_send(Event::Completed);
    let cmd_three = Command::request_from_shell(AnOperation::One).then_send(Event::Completed);

    let cmds = vec![cmd_one, cmd_two, cmd_three];

    let mut cmd: Command<Effect, Event> = cmds.into_iter().collect();

    assert!(cmd.events().next().is_none());

    let mut effects: Vec<_> = cmd.effects().collect();

    assert_eq!(effects.len(), 3);

    let Effect::AnEffect(mut request) = effects.remove(0);

    assert_eq!(request.operation, AnOperation::One);

    request
        .resolve(AnOperationOutput::One)
        .expect("request should resolve");

    // Still the original effects
    let Effect::AnEffect(mut request) = effects.remove(0);

    assert_eq!(request.operation, AnOperation::Two);

    request
        .resolve(AnOperationOutput::Two)
        .expect("request should resolve");

    assert!(cmd.effects().next().is_none());

    // Still the original effects
    let Effect::AnEffect(mut request) = effects.remove(0);

    assert_eq!(request.operation, AnOperation::One);

    request
        .resolve(AnOperationOutput::Two)
        .expect("request should resolve");

    assert!(cmd.effects().next().is_none());

    let events: Vec<_> = cmd.events().collect();

    assert_eq!(events[0], Event::Completed(AnOperationOutput::One));
    assert_eq!(events[1], Event::Completed(AnOperationOutput::Two));
    assert_eq!(events[1], Event::Completed(AnOperationOutput::Two));

    assert!(cmd.is_done());
}

#[test]
fn complex_concurrency() {
    fn increment(output: AnOperationOutput) -> AnOperation {
        let AnOperationOutput::Other([a, b]) = output else {
            panic!("bad output");
        };

        AnOperation::More([a, b + 1])
    }

    let mut cmd = Command::all([
        Command::request_from_shell(AnOperation::More([1, 1]))
            .then_request(|out| Command::request_from_shell(increment(out)))
            .then_send(Event::Completed),
        Command::request_from_shell(AnOperation::More([2, 1]))
            .then_request(|out| Command::request_from_shell(increment(out)))
            .then_send(Event::Completed),
    ])
    .then(Command::request_from_shell(AnOperation::More([3, 1])).then_send(Event::Completed));

    // Phase 1

    assert!(cmd.events().next().is_none());
    let mut effects: Vec<_> = cmd.effects().collect();

    assert_eq!(effects.len(), 2);

    let Effect::AnEffect(mut request_1) = effects.remove(0);
    let Effect::AnEffect(mut request_2) = effects.remove(0);

    assert_eq!(request_1.operation, AnOperation::More([1, 1]));
    assert_eq!(request_2.operation, AnOperation::More([2, 1]));

    request_1
        .resolve(AnOperationOutput::Other([1, 1]))
        .expect("request should resolve");

    request_2
        .resolve(AnOperationOutput::Other([2, 1]))
        .expect("request should resolve");

    // Phase 2

    assert!(cmd.events().next().is_none());
    let mut effects: Vec<_> = cmd.effects().collect();

    assert_eq!(effects.len(), 2);

    let Effect::AnEffect(mut request_1) = effects.remove(0);
    let Effect::AnEffect(mut request_2) = effects.remove(0);

    assert_eq!(request_1.operation, AnOperation::More([1, 2]));
    assert_eq!(request_2.operation, AnOperation::More([2, 2]));

    request_1
        .resolve(AnOperationOutput::Other([1, 2]))
        .expect("request should resolve");

    request_2
        .resolve(AnOperationOutput::Other([2, 2]))
        .expect("request should resolve");

    // Phase 3

    let events: Vec<_> = cmd.events().collect();
    let mut effects: Vec<_> = cmd.effects().collect();

    assert_eq!(events.len(), 2);

    assert_eq!(
        events[0],
        Event::Completed(AnOperationOutput::Other([1, 2]))
    );
    assert_eq!(
        events[1],
        Event::Completed(AnOperationOutput::Other([2, 2]))
    );

    assert_eq!(effects.len(), 1);

    let Effect::AnEffect(mut request_1) = effects.remove(0);

    assert_eq!(request_1.operation, AnOperation::More([3, 1]));

    request_1
        .resolve(AnOperationOutput::Other([3, 1]))
        .expect("request should resolve");

    // Phase 4

    let events: Vec<_> = cmd.events().collect();

    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0],
        Event::Completed(AnOperationOutput::Other([3, 1]))
    );

    assert!(cmd.is_done());
}

#[test]
fn concurrency_mixing_streams_and_requests() {
    let mut cmd = Command::all([
        Command::stream_from_shell(AnOperation::One)
            .then_request(|out| {
                let AnOperationOutput::Other([a, b]) = out else {
                    panic!("Bad output");
                };

                Command::request_from_shell(AnOperation::More([a + 1, b + 1]))
            })
            .then_send(Event::Completed),
        Command::request_from_shell(AnOperation::Two)
            .then_stream(|out| {
                let AnOperationOutput::Other([a, b]) = out else {
                    panic!("Bad output");
                };

                Command::stream_from_shell(AnOperation::More([a + 2, b + 2]))
            })
            .then_send(Event::Completed),
    ]);

    assert!(cmd.events().next().is_none());
    let mut effects: Vec<_> = cmd.effects().collect();

    assert_eq!(effects.len(), 2);

    let Effect::AnEffect(mut stream_request) = effects.remove(0);
    let Effect::AnEffect(mut request) = effects.remove(0);

    assert_eq!(stream_request.operation, AnOperation::One);
    assert_eq!(request.operation, AnOperation::Two);

    stream_request
        .resolve(AnOperationOutput::Other([1, 2]))
        .expect("should resolve");

    let mut effects: Vec<_> = cmd.effects().collect();

    let Effect::AnEffect(mut plus_one_request) = effects.remove(0);
    assert_eq!(plus_one_request.operation, AnOperation::More([2, 3]));

    plus_one_request
        .resolve(AnOperationOutput::One)
        .expect("should resolve");

    let events: Vec<_> = cmd.events().collect();
    assert_eq!(events[0], Event::Completed(AnOperationOutput::One));

    // Can't request the plus one request again
    assert!(plus_one_request.resolve(AnOperationOutput::One).is_err());

    // but can get a new one by resolving stream request again
    stream_request
        .resolve(AnOperationOutput::Other([2, 3]))
        .expect("should resolve");

    let effect = cmd.effects().next().unwrap();

    let Effect::AnEffect(plus_one_request) = effect;
    assert_eq!(plus_one_request.operation, AnOperation::More([3, 4]));

    // The second request is the opposite

    request
        .resolve(AnOperationOutput::Other([1, 2]))
        .expect("should resolve");
    assert!(request.resolve(AnOperationOutput::Other([1, 2])).is_err());

    let effect = cmd.effects().next().unwrap();

    let Effect::AnEffect(mut plus_two_request) = effect;

    assert_eq!(plus_two_request.operation, AnOperation::More([3, 4]));

    // Plus two request is a subscription

    plus_two_request
        .resolve(AnOperationOutput::One)
        .expect("should resolve");
    plus_two_request
        .resolve(AnOperationOutput::Two)
        .expect("should resolve");
    plus_two_request
        .resolve(AnOperationOutput::One)
        .expect("should resolve");

    let events: Vec<_> = cmd.events().collect();
    assert_eq!(events[0], Event::Completed(AnOperationOutput::One));
    assert_eq!(events[1], Event::Completed(AnOperationOutput::Two));
    assert_eq!(events[2], Event::Completed(AnOperationOutput::One));
}

#[test]
fn stream_followed_by_a_stream() {
    let mut cmd = Command::stream_from_shell(AnOperation::One)
        .then_stream(|out| {
            let AnOperationOutput::Other([a, b]) = out else {
                panic!("Bad output");
            };

            Command::stream_from_shell(AnOperation::More([a + 1, b + 1]))
        })
        .then_send(Event::Completed);

    let effect = cmd.effects().next().unwrap();
    let Effect::AnEffect(mut spawner_request) = effect;

    assert!(cmd.effects().next().is_none());
    assert!(cmd.events().next().is_none());

    // This is a bit of a mind bend. Every time we resolve the `spawner_request` we receive a _new_
    // stream request.
    //
    // Resolving any of those requests should all result in an event immediately

    // 1. resolve the initial stream three times

    let mut streams = [1, 2, 3].map(|i| {
        spawner_request
            .resolve(AnOperationOutput::Other([i, i + 1]))
            .expect("should resolve");

        let effect = cmd.effects().next().unwrap();
        let Effect::AnEffect(stream) = effect;

        // Check the then logic has applied
        assert_eq!(stream.operation, AnOperation::More([i + 1, i + 2]));

        stream
    });

    // 2. now every time we resolve the stream, the next event we get
    // is triggered by this stream.

    for i in 1..=5 {
        for stream in &mut streams {
            assert!(cmd.events().next().is_none());

            let AnOperation::More([a, b]) = stream.operation else {
                panic!();
            };

            let resolved_with = [a + i, b + i];

            stream
                .resolve(AnOperationOutput::Other(resolved_with))
                .expect("should resolve");

            assert_eq!(
                Event::Completed(AnOperationOutput::Other(resolved_with)),
                cmd.events().next().unwrap()
            )
        }
    }

    assert!(cmd.events().next().is_none())
}

#[test]
fn chaining_with_mapping() {
    let mut cmd = Command::request_from_shell(AnOperation::More([3, 4]))
        .map(|first| {
            let AnOperationOutput::Other(first) = first else {
                // TODO: how do I bail quietly here?
                panic!("Invalid output!")
            };

            first
        })
        .then_request(|first| {
            let second = [first[0] + 1, first[1] + 1];

            Command::request_from_shell(AnOperation::More(second))
        })
        .then_send(Event::Completed);

    let effect = cmd.effects().next().unwrap();
    assert!(cmd.events().next().is_none());

    let Effect::AnEffect(mut request) = effect;

    assert_eq!(request.operation, AnOperation::More([3, 4]));
    request
        .resolve(AnOperationOutput::Other([1, 2]))
        .expect("to resolve");

    let effect = cmd.effects().next().unwrap();
    assert!(cmd.events().next().is_none());

    let Effect::AnEffect(mut request) = effect;
    assert_eq!(request.operation, AnOperation::More([2, 3]));

    request
        .resolve(AnOperationOutput::Other([1, 2]))
        .expect("to resolve");

    let event = cmd.events().next().unwrap();
    assert!(cmd.effects().next().is_none());

    assert_eq!(event, Event::Completed(AnOperationOutput::Other([1, 2])));

    assert!(cmd.is_done());
}

#[test]
fn stream_mapping_and_chaining() {
    let mut cmd = Command::stream_from_shell(AnOperation::One)
        .map(|out| {
            let AnOperationOutput::Other([a, b]) = out else {
                panic!("Bad output");
            };

            (a, b)
        })
        .then_request(|(a, b)| Command::request_from_shell(AnOperation::More([a + 1, b + 1])))
        .then_send(Event::Completed);

    assert!(cmd.events().next().is_none());
    let mut effects: Vec<_> = cmd.effects().collect();

    assert_eq!(effects.len(), 1);

    let Effect::AnEffect(mut stream_request) = effects.remove(0);

    assert_eq!(stream_request.operation, AnOperation::One);

    stream_request
        .resolve(AnOperationOutput::Other([1, 2]))
        .expect("should resolve");

    let mut effects: Vec<_> = cmd.effects().collect();

    let Effect::AnEffect(mut plus_one_request) = effects.remove(0);
    assert_eq!(plus_one_request.operation, AnOperation::More([2, 3]));

    plus_one_request
        .resolve(AnOperationOutput::One)
        .expect("should resolve");

    let events: Vec<_> = cmd.events().collect();
    assert_eq!(events[0], Event::Completed(AnOperationOutput::One));

    // Can't request the plus one request again
    assert!(plus_one_request.resolve(AnOperationOutput::One).is_err());

    // but can get a new one by resolving stream request again
    stream_request
        .resolve(AnOperationOutput::Other([2, 3]))
        .expect("should resolve");

    let effect = cmd.effects().next().unwrap();

    let Effect::AnEffect(plus_one_request) = effect;
    assert_eq!(plus_one_request.operation, AnOperation::More([3, 4]));
}
