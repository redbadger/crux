use futures::{Stream, StreamExt as _};
use std::future::Future;

use serde::{Deserialize, Serialize};

use super::super::Command;
use crate::{
    capability::Operation,
    command::builder::{RequestBuilder, StreamBuilder},
    Request,
};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
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

// This Capability example is really contrived

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

            assert!(!cmd.is_done());
        }
    }
}
