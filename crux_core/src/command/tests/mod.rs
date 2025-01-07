mod basic_effects {
    use serde::{Deserialize, Serialize};

    use super::super::Command;
    use crate::{capability::Operation, Request};

    #[derive(Debug, PartialEq, Clone, Serialize)]
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
        let cmd: Command<Effect, Event> = Command::done();

        assert!(cmd.is_done())
    }

    #[test]
    fn notify_can_be_created_with_an_operation() {
        let cmd: Command<Effect, Event> = Command::notify_shell(AnOperation);

        assert!(!cmd.is_done())
    }

    #[test]
    fn notify_effect_can_be_inspected() {
        let mut cmd: Command<_, Event> = Command::notify_shell(AnOperation);

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

        assert!(matches!(event, Event::Completed(AnOperationOutput)));

        assert!(cmd.is_done())
    }

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
}

mod async_effects {
    use futures::{join, select, FutureExt};
    use serde::{Deserialize, Serialize};

    use super::super::Command;
    use crate::{capability::Operation, Request};

    #[derive(Debug, PartialEq, Clone, Serialize)]
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
        Abort,
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
        let mut cmd: Command<Effect, Event> = Command::new(|ctx| async move {
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
        let mut cmd: Command<Effect, Event> = Command::new(|ctx| async move {
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
        let mut cmd: Command<Effect, Event> = Command::new(|ctx| async move {
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

        let mut cmd: Command<Effect, Event> = Command::new(|ctx| async move {
            let (tx, rx) = async_channel::unbounded();

            ctx.spawn({
                let ctx = ctx.clone();
                async move {
                    loop {
                        let output = ctx.request_from_shell(AnOperation::One).await;

                        if output == AnOperationOutput::Abort {
                            break;
                        }

                        tx.send(output).await.unwrap();
                    }
                }
            });

            ctx.spawn({
                let ctx = ctx.clone();
                async move {
                    while let Ok(value) = rx.recv().await {
                        ctx.send_event(Event::Completed(value));
                    }

                    ctx.send_event(Event::Aborted);
                }
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

        let Effect::AnEffect(mut first_request) = effects.remove(0);
        first_request
            .resolve(AnOperationOutput::Abort)
            .expect("request should resolve");

        assert!(cmd.effects().next().is_none());

        assert_eq!(cmd.events().next().unwrap(), Event::Aborted);

        assert!(cmd.is_done());
    }
}

mod combinators {
    use serde::{Deserialize, Serialize};

    use super::super::Command;
    use crate::{capability::Operation, command::builder::CommandBuilder, Request};

    #[derive(Debug, PartialEq, Clone, Serialize)]
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
        let mut cmd: Command<Effect, Event> =
            Command::request_from_shell(AnOperation::More([3, 4]))
                .then(|first| {
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
    fn complex_concurrency() {
        fn increment(output: AnOperationOutput) -> AnOperation {
            let AnOperationOutput::Other([a, b]) = output else {
                panic!("bad output");
            };

            AnOperation::More([a, b + 1])
        }

        let mut cmd = Command::all([
            Command::request_from_shell(AnOperation::More([1, 1]))
                .then(|out| Command::request_from_shell(increment(out)))
                .then_send(Event::Completed),
            Command::request_from_shell(AnOperation::More([2, 1]))
                .then(|out| Command::request_from_shell(increment(out)))
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
        let mut cmd: Command<Effect, Event> = Command::all([
            Command::stream_from_shell(AnOperation::One)
                .then(|out| {
                    let AnOperationOutput::Other([a, b]) = out else {
                        panic!("Bad output");
                    };

                    Command::request_from_shell(AnOperation::More([a + 1, b + 1]))
                })
                .then_send(Event::Completed),
            Command::request_from_shell(AnOperation::Two)
                .then(|out| {
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
}

mod capability_api {
    use futures::{Stream, StreamExt as _};
    use std::future::Future;

    use serde::{Deserialize, Serialize};

    use super::super::Command;
    use crate::{
        capability::Operation,
        command::builder::{RequestBuilder, StreamBuilder},
        Request,
    };

    #[derive(Debug, PartialEq, Clone, Serialize)]
    enum AnOperation {
        Request(usize),
        Stream(String),
    }

    #[derive(Debug, PartialEq, Deserialize)]
    enum AnOperationOutput {
        Response(String),
        StreamEvent { order: usize, message: String },
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

    // This Capability exampls is really contrived

    struct Capability;

    // FIXME: can the return types be made less verbose...?
    impl Capability
    where
        Effect: Send + 'static,
        Event: Send + 'static,
    {
        fn request(
            number: usize,
        ) -> RequestBuilder<Effect, Event, impl Future<Output = AnOperationOutput>> {
            Command::request_from_shell(AnOperation::Request(number))
        }

        fn stream(
            name: impl Into<String>,
        ) -> StreamBuilder<Effect, Event, impl Stream<Item = AnOperationOutput>> {
            Command::stream_from_shell(AnOperation::Stream(name.into()))
        }
    }

    #[test]
    fn request() {
        // Sync API
        let sync_cmd = Capability::request(10).then_send(Event::Completed);

        // Async API
        let async_cmd = Command::new(|ctx| async move {
            let out = Capability::request(10).into_future(ctx.clone()).await;

            ctx.send_event(Event::Completed(out));
        });

        for mut cmd in [sync_cmd, async_cmd] {
            let effect = cmd.effects().next().unwrap();
            assert!(cmd.events().next().is_none());

            let Effect::AnEffect(mut request) = effect;

            assert_eq!(request.operation, AnOperation::Request(10));

            request
                .resolve(AnOperationOutput::Response("ten".to_string()))
                .expect("should work");

            let event = cmd.events().next().unwrap();

            assert_eq!(
                event,
                Event::Completed(AnOperationOutput::Response("ten".to_string()))
            );

            assert!(cmd.is_done());
        }
    }

    #[test]
    fn stream_event() {
        // Sync API
        let sync_cmd = Capability::stream("hello").then_send(Event::Completed);

        // Async API
        let async_cmd = Command::new(|ctx| async move {
            let mut stream = Capability::stream("hello").into_stream(ctx.clone());

            while let Some(out) = stream.next().await {
                ctx.send_event(Event::Completed(out));
            }
        });

        for mut cmd in [sync_cmd, async_cmd] {
            let effect = cmd.effects().next().unwrap();

            let Effect::AnEffect(mut request) = effect;

            for order in 1..10 {
                assert_eq!(request.operation, AnOperation::Stream("hello".to_string()));

                request
                    .resolve(AnOperationOutput::StreamEvent {
                        order,
                        message: "Hi".to_string(),
                    })
                    .expect("should work");

                let event = cmd.events().next().unwrap();

                assert_eq!(
                    event,
                    Event::Completed(AnOperationOutput::StreamEvent {
                        order,
                        message: "Hi".to_string()
                    })
                );

                assert!(!cmd.is_done())
            }
        }
    }
}

mod composition {
    use serde::Serialize;

    use crate::{capability::Operation, Command, Request};

    #[derive(Debug, PartialEq, Clone, Serialize)]
    struct ToString(usize);

    impl Operation for ToString {
        type Output = String;
    }

    #[derive(PartialEq, Clone, Serialize)]
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
}

mod cancellation {
    use serde::Serialize;

    use crate::{capability::Operation, Request};

    use super::super::Command;

    #[derive(Debug, Clone, PartialEq, Serialize)]
    struct Op;

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
                    ctx.request_from_shell(Op).await;
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
}
