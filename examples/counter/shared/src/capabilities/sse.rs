use async_sse::{decode, Event};
use async_std::io::Cursor;
use futures::StreamExt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crux_core::capability::{CapabilityContext, Operation};

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
pub struct ServerSentEvents<Ev> {
    context: CapabilityContext<SseRequest, Ev>,
}

impl<Ev> ServerSentEvents<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<SseRequest, Ev>) -> Self {
        Self { context }
    }

    pub fn get_json<F, T>(&self, url: impl AsRef<str>, make_event: F)
    where
        F: Fn(T) -> Ev + Clone + Send + 'static,
        T: DeserializeOwned,
    {
        self.context.spawn({
            let context = self.context.clone();
            let url = url.as_ref().to_string();

            async move {
                let mut stream = context.stream_from_shell(SseRequest { url });

                while let Some(response) = stream.next().await {
                    let make_event = make_event.clone();

                    match response {
                        SseResponse::Chunk(data) => {
                            let mut reader = decode(Cursor::new(data));

                            while let Some(sse_event) = reader.next().await {
                                if let Ok(Event::Message(msg)) = sse_event {
                                    let t: T = serde_json::from_slice(msg.data()).unwrap();
                                    context.update_app(make_event(t));
                                }
                            }
                        }
                        SseResponse::Done => break,
                    }
                }
            }
        });
    }
}
