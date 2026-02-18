use std::{convert::From, future};

use async_sse::{Event as SseEvent, decode};
use async_std::io::Cursor;
use facet::Facet;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crux_core::{Request, capability::Operation, command::StreamBuilder};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SseRequest {
    pub url: String,
}

#[derive(Facet, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum SseResponse {
    Chunk(Vec<u8>),
    Done,
}

impl SseResponse {
    #[must_use]
    pub const fn is_done(&self) -> bool {
        matches!(self, Self::Done)
    }
}

impl Operation for SseRequest {
    type Output = SseResponse;
}

pub struct ServerSentEvents;

impl ServerSentEvents {
    pub fn get<Effect, Event, T>(
        url: impl Into<String>,
    ) -> StreamBuilder<Effect, Event, impl Stream<Item = T>>
    where
        Effect: From<Request<SseRequest>> + Send + 'static,
        Event: Send + 'static,
        T: Send + DeserializeOwned,
    {
        let url = url.into();

        StreamBuilder::new(|ctx| {
            ctx.stream_from_shell(SseRequest { url })
                .take_while(|response| future::ready(!response.is_done()))
                .flat_map(|response| {
                    let SseResponse::Chunk(data) = response else {
                        unreachable!()
                    };

                    decode(Cursor::new(data))
                })
                .filter_map(|sse_event| async {
                    sse_event.ok().and_then(|event| match event {
                        SseEvent::Message(msg) => serde_json::from_slice(msg.data()).ok(),
                        SseEvent::Retry(_) => None, // do we need to worry about this?
                    })
                })
        })
    }
}
