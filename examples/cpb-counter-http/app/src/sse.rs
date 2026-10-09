//! `counter_http`'s `ServerSentEvents` command, without std.
//!
//! `counter_http` decodes each chunk on its own with `async-sse` (std). Here a small
//! parser keeps a buffer across chunks, because an event can be split between them:
//! the gateway forwards fetch's chunks as they come.

use alloc::vec::Vec;
use core::future;

use crux_core::{Request, command::StreamBuilder};
use futures_util::{Stream, StreamExt, stream};
use serde::de::DeserializeOwned;

use ble_protocol::{SseRequest, SseResponse};

pub struct ServerSentEvents;

impl ServerSentEvents {
    /// Open the stream at `url`, and yield each message's `data` deserialized from JSON
    /// (messages that are not a `T` are skipped).
    pub fn get<Effect, Event, T>(
        url: impl Into<alloc::string::String>,
    ) -> StreamBuilder<Effect, Event, impl Stream<Item = T>>
    where
        Effect: From<Request<SseRequest>> + Send + 'static,
        Event: Send + 'static,
        T: Send + DeserializeOwned + 'static,
    {
        let url = url.into();

        StreamBuilder::new(|ctx| {
            let mut parser = Parser::default();
            ctx.stream_from_shell(SseRequest { url })
                .take_while(|response| future::ready(!response.is_done()))
                .flat_map(move |response| {
                    let SseResponse::Chunk(data) = response else {
                        unreachable!()
                    };
                    stream::iter(parser.push(&data))
                })
                .filter_map(|data| future::ready(serde_json::from_slice(&data).ok()))
        })
    }
}

/// The subset of the SSE wire format the counter needs.
///
/// `data:` lines are joined with newlines and dispatched on a blank line. Other fields (`event:`, `id:`, `retry:`) and
/// comments are ignored. Lines may end in `\n` or `\r\n`.
#[derive(Default)]
pub struct Parser {
    /// Bytes after the last complete line.
    partial: Vec<u8>,
    /// The `data` of the message being read.
    data: Vec<u8>,
    has_data: bool,
}

impl Parser {
    /// Add a chunk; return the `data` of every message it completes.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<Vec<u8>> {
        self.partial.extend_from_slice(chunk);
        let mut messages = Vec::new();

        while let Some(end) = self.partial.iter().position(|&byte| byte == b'\n') {
            let mut line: Vec<u8> = self.partial.drain(..=end).collect();
            line.pop(); // '\n'
            if line.last() == Some(&b'\r') {
                line.pop();
            }

            if line.is_empty() {
                if self.has_data {
                    messages.push(core::mem::take(&mut self.data));
                    self.has_data = false;
                }
            } else if let Some(value) = line.strip_prefix(b"data:") {
                let value = value.strip_prefix(b" ").unwrap_or(value);
                if self.has_data {
                    self.data.push(b'\n');
                }
                self.data.extend_from_slice(value);
                self.has_data = true;
            }
        }
        messages
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::Parser;

    #[test]
    fn one_message_per_blank_line() {
        let mut parser = Parser::default();
        assert_eq!(
            parser.push(b"data: {\"value\":1}\n\ndata: {\"value\":2}\n\n"),
            vec![b"{\"value\":1}".to_vec(), b"{\"value\":2}".to_vec()]
        );
    }

    #[test]
    fn a_message_split_across_chunks() {
        let mut parser = Parser::default();
        assert_eq!(parser.push(b"da"), [] as [alloc::vec::Vec<u8>; 0]);
        assert_eq!(parser.push(b"ta: {\"val"), [] as [alloc::vec::Vec<u8>; 0]);
        assert_eq!(parser.push(b"ue\":3}\r\n"), [] as [alloc::vec::Vec<u8>; 0]);
        assert_eq!(parser.push(b"\r\n"), vec![b"{\"value\":3}".to_vec()]);
    }

    #[test]
    fn multi_line_data_and_ignored_fields() {
        let mut parser = Parser::default();
        assert_eq!(
            parser.push(b": comment\nevent: count\nid: 7\ndata: a\ndata:b\nretry: 10\n\n\n"),
            vec![b"a\nb".to_vec()]
        );
    }
}
