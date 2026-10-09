//! The gateway's Server-Sent Events capability.
//!
//! The device asks with `ble_protocol`'s `SseRequest`, and only learns that a stream ended
//! (`SseDone`). The gateway wants to say *how* it ended in its log, so it asks its shell
//! with an operation of its own, whose stream ends with either `Closed` or `Failed`.

use crux_core::macros::Operation;

/// Open the Server-Sent Events stream at `url`.
///
/// The shell answers with a stream: a `Chunk` per read of the response body, then exactly
/// one `Closed` (the body ended) or `Failed` (the fetch was rejected, the status was not
/// 2xx, or reading the body failed).
#[derive(Operation, Debug, Clone, PartialEq, Eq)]
#[operation(stream, output = SseEvent)]
pub struct SseStream {
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SseEvent {
    Chunk(Vec<u8>),
    /// The server ended the response.
    Closed,
    /// The stream never opened, or broke; why.
    Failed(String),
}
