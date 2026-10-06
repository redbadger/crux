//! The Server-Sent Events operation, as in counter-http's `sse.rs`, without std.
//!
//! Only the operation lives here, because the firmware app and the gateway both need it.
//! The app's `ServerSentEvents::get` command (which parses the stream) is in the app crate.

use alloc::{string::String, vec::Vec};

use crux_core::macros::Operation;
use serde::{Deserialize, Serialize};

/// Open a Server-Sent Events stream on `url`. The shell answers with a sequence of
/// [`SseResponse`]s: a `Chunk` per batch of bytes it reads, then one `Done` when the
/// server closes the connection.
#[derive(Operation, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[operation(stream, output = SseResponse)]
pub struct SseRequest {
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum SseResponse {
    Chunk(#[serde(with = "serde_bytes")] Vec<u8>),
    Done,
}

impl SseResponse {
    #[must_use]
    pub const fn is_done(&self) -> bool {
        matches!(self, Self::Done)
    }
}
