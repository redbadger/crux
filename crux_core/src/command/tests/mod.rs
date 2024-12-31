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

        let mut effects = cmd.effects();

        assert!(!effects.is_empty());

        let Effect::AnEffect(request) = effects.remove(0);

        assert_eq!(request.operation, AnOperation)
    }

    #[test]
    fn request_effect_can_be_inspected() {
        let mut cmd = Command::request_from_shell(AnOperation, Event::Completed);

        let mut effects = cmd.effects();

        assert!(!effects.is_empty());

        let Effect::AnEffect(request) = effects.remove(0);

        assert_eq!(request.operation, AnOperation)
    }

    #[test]
    fn request_effect_can_be_resolved() {
        let mut cmd = Command::request_from_shell(AnOperation, Event::Completed);

        let mut effects = cmd.effects();

        assert!(cmd.events().is_empty());

        let Effect::AnEffect(mut request) = effects.remove(0);

        assert_eq!(request.operation, AnOperation);

        request
            .resolve(AnOperationOutput)
            .expect("Resolve should succeed");

        let mut events = cmd.events();

        assert!(matches!(
            events.remove(0),
            Event::Completed(AnOperationOutput)
        ));

        assert!(cmd.is_done())
    }

    #[test]
    fn stream_effect_can_be_resolved_multiple_times() {
        let mut cmd = Command::stream_from_shell(AnOperation, Event::Completed);

        let mut effects = cmd.effects();

        assert!(cmd.events().is_empty());

        let Effect::AnEffect(mut request) = effects.remove(0);

        assert_eq!(request.operation, AnOperation);

        request
            .resolve(AnOperationOutput)
            .expect("Resolve should succeed");

        let mut events = cmd.events();

        assert!(matches!(
            events.remove(0),
            Event::Completed(AnOperationOutput)
        ));

        assert!(cmd.effects().is_empty());
        assert!(cmd.events().is_empty());
        assert!(!cmd.is_done());

        request
            .resolve(AnOperationOutput)
            .expect("Resolve should succeed");

        let mut events = cmd.events();

        assert!(matches!(
            events.remove(0),
            Event::Completed(AnOperationOutput)
        ));
    }

    #[test]
    fn event_can_be_created() {
        let mut cmd: Command<Effect, _> = Command::event(Event::Start);

        let events = cmd.events();

        assert_eq!(events[0], Event::Start);
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
    }
    #[derive(Debug, PartialEq, Deserialize)]
    enum AnOperationOutput {
        One,
        Two,
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

        assert!(cmd.events().is_empty());

        let mut effects = cmd.effects();
        let Effect::AnEffect(mut request) = effects.remove(0);

        assert_eq!(request.operation, AnOperation::One);

        request
            .resolve(AnOperationOutput::One)
            .expect("request should resolve");

        let event = cmd.events().remove(0);

        assert_eq!(event, Event::Completed(AnOperationOutput::One));

        assert!(cmd.events().is_empty());

        let mut effects = cmd.effects();
        let Effect::AnEffect(mut request) = effects.remove(0);

        assert_eq!(request.operation, AnOperation::Two);

        request
            .resolve(AnOperationOutput::Two)
            .expect("request should resolve");

        assert!(cmd.effects().is_empty());

        let event = cmd.events().remove(0);

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

        assert!(cmd.events().is_empty());

        let mut effects = cmd.effects();
        let Effect::AnEffect(mut first_request) = effects.remove(0);
        let Effect::AnEffect(mut second_request) = effects.remove(0);

        assert_eq!(first_request.operation, AnOperation::One);
        assert_eq!(second_request.operation, AnOperation::Two);

        first_request
            .resolve(AnOperationOutput::One)
            .expect("request should resolve");

        assert!(cmd.events().is_empty());

        second_request
            .resolve(AnOperationOutput::Two)
            .expect("request should resolve");

        assert!(cmd.effects().is_empty());

        let mut events = cmd.events();

        assert_eq!(events.len(), 2);

        assert_eq!(events.remove(0), Event::Completed(AnOperationOutput::One));
        assert_eq!(events.remove(0), Event::Completed(AnOperationOutput::Two));
    }

    #[test]
    fn effects_race() {
        let mut cmd: Command<Effect, Event> = Command::new(|ctx| async move {
            select! {
                output = ctx.request_from_shell(AnOperation::One).fuse() => ctx.send_event(Event::Completed(output)),
                output = ctx.request_from_shell(AnOperation::Two).fuse() => ctx.send_event(Event::Completed(output))
            };
        });

        assert!(cmd.events().is_empty());

        let mut effects = cmd.effects();
        let Effect::AnEffect(mut first_request) = effects.remove(0);
        let Effect::AnEffect(mut second_request) = effects.remove(0);

        assert_eq!(first_request.operation, AnOperation::One);
        assert_eq!(second_request.operation, AnOperation::Two);

        second_request
            .resolve(AnOperationOutput::Two)
            .expect("request should resolve");

        first_request
            .resolve(AnOperationOutput::Two)
            .expect("request should resolve");

        let mut events = cmd.events();

        assert_eq!(events.len(), 1);
        assert_eq!(events.remove(0), Event::Completed(AnOperationOutput::Two));
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
            })
        });

        let mut effects = cmd.effects();

        assert_eq!(effects.len(), 1);

        let Effect::AnEffect(mut first_request) = effects.remove(0);
        first_request
            .resolve(AnOperationOutput::One)
            .expect("request should resolve");

        let mut effects = cmd.effects();
        let events = cmd.events();

        assert_eq!(effects.len(), 1);
        assert_eq!(events.len(), 1);

        assert_eq!(events[0], Event::Completed(AnOperationOutput::One));

        let Effect::AnEffect(mut first_request) = effects.remove(0);
        first_request
            .resolve(AnOperationOutput::Two)
            .expect("request should resolve");

        let mut effects = cmd.effects();
        let events = cmd.events();

        assert_eq!(effects.len(), 1);
        assert_eq!(events.len(), 1);

        assert_eq!(events[0], Event::Completed(AnOperationOutput::Two));

        let Effect::AnEffect(mut first_request) = effects.remove(0);
        first_request
            .resolve(AnOperationOutput::Abort)
            .expect("request should resolve");

        assert!(cmd.effects().is_empty());

        assert_eq!(cmd.events()[0], Event::Aborted);

        assert!(cmd.is_done());
    }
}
