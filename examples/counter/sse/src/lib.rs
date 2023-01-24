use async_sse::{decode, Event};
use async_std::io::Cursor;
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use crux_core::{
    capability::{CapabilityContext, Operation},
    Capability,
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

    pub fn get<F>(&self, url: &str, event: F)
    where
        F: Fn(String) -> Ev + Clone + Send + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            let url = url.to_string();

            async move {
                let mut stream = context.stream_from_shell(SseRequest { url });

                while let Some(maybe_response) = stream.next().await {
                    let event = event.clone();

                    match maybe_response {
                        SseResponse::Chunk(data) => {
                            let mut reader = decode(Cursor::new(data));
                            while let Some(Ok(Event::Message(msg))) = reader.next().await {
                                let data = String::from_utf8(msg.data().to_owned()).unwrap();
                                context.update_app(event(data));
                            }
                        }
                        _ => break,
                    }
                }
            }
        });
    }
}

impl<Ef> Capability<Ef> for ServerSentEvents<Ef> {
    type Operation = SseRequest;
    type MappedSelf<MappedEv> = ServerSentEvents<MappedEv>;

    fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
    where
        F: Fn(NewEvent) -> Ef + Send + Sync + Copy + 'static,
        Ef: 'static,
        NewEvent: 'static,
    {
        ServerSentEvents::new(self.context.map_event(f))
    }
}
