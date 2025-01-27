use std::convert::From;

use async_sse::{decode, Event as SseEvent};
use async_std::io::Cursor;
use futures::StreamExt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crux_core::{
    capability::{CapabilityContext, Operation},
    Command, Request,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SseRequest {
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum SseResponse {
    Chunk(Vec<u8>),
    Done,
}

impl Operation for SseRequest {
    type Output = SseResponse;
}

#[derive(crux_core::macros::Capability)]
pub struct ServerSentEvents<Event> {
    context: CapabilityContext<SseRequest, Event>,
}

impl<Event> ServerSentEvents<Event>
where
    Event: Send + 'static,
{
    pub fn new(context: CapabilityContext<SseRequest, Event>) -> Self {
        Self { context }
    }

    pub fn get_json<Effect, F, T>(url: impl AsRef<str>, make_event: F) -> Command<Effect, Event>
    where
        Effect: From<Request<SseRequest>> + Send + 'static,
        F: Fn(T) -> Event + Clone + Send + 'static,
        T: DeserializeOwned,
    {
        let url = url.as_ref().to_string();

        Command::new(|ctx| async move {
            let mut stream =
                Command::stream_from_shell(SseRequest { url }).into_stream(ctx.clone());

            while let Some(response) = stream.next().await {
                match response {
                    SseResponse::Chunk(data) => {
                        let mut reader = decode(Cursor::new(data));

                        while let Some(sse_event) = reader.next().await {
                            if let Ok(SseEvent::Message(msg)) = sse_event {
                                let t: T = serde_json::from_slice(msg.data()).unwrap();
                                ctx.send_event(make_event(t));
                            }
                        }
                    }
                    SseResponse::Done => break,
                }
            }
        })
    }
}
