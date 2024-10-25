use async_sse::{decode, Event};
use async_std::io::Cursor;
use futures::StreamExt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crux_core::capability::{CapabilityContext, Operation};
use crux_core::Command;

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
pub struct ServerSentEvents {
    context: CapabilityContext<SseRequest>,
}

impl ServerSentEvents {
    pub fn new(context: CapabilityContext<SseRequest>) -> Self {
        Self { context }
    }

    pub fn get_json<F, Ev, T>(&self, url: impl AsRef<str>, make_event: F) -> Command<Ev>
    where
        F: Fn(T) -> Ev + Clone + Send + 'static,
        Ev: Send + 'static,
        T: DeserializeOwned,
    {
        let context = self.context.clone();
        let url = url.as_ref().to_string();

        let stream = context
            .stream_from_shell(SseRequest { url })
            .flat_map(move |response| match response {
                SseResponse::Chunk(data) => {
                    // this decoder is async (even though for our purposed it doesn't need to be)
                    // which makes the following code a bit fiddly
                    let reader = decode(Cursor::new(data));
                    let make_event = make_event.clone();
                    let inner = reader.map(move |sse_event| {
                        if let Ok(Event::Message(msg)) = sse_event {
                            let t: T = serde_json::from_slice(msg.data()).unwrap();
                            let make_event = make_event.clone();
                            Command::event(make_event(t))
                        } else {
                            Command::none()
                        }
                    });
                    Box::pin(inner) as futures::stream::BoxStream<_>
                }
                SseResponse::Done => Box::pin(futures::stream::empty()),
            });
        Command::stream(stream)
    }
}
