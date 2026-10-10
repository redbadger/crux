//! `spike(no_std)`: the link between the CPB firmware and the Chrome gateway.
//!
//! The device is the GATT peripheral (Web Bluetooth can only be a central). It exposes
//! one service with two characteristics: [`TX_UUID`] (notify, device → gateway) and
//! [`RX_UUID`] (write without response, gateway → device).
//!
//! Each direction carries a stream of messages: [`ToGateway`] one way, [`ToDevice`] the
//! other. A message is postcard-encoded and framed with a little-endian `u16` length
//! prefix ([`encode`]), then cut into chunks no larger than the ATT payload ([`chunks`]).
//! The receiver feeds every chunk to a [`Reassembler`], which yields whole messages.
//! Chunk boundaries carry no meaning, so each side can use whatever size its link allows.
//!
//! HTTP travels as `crux_http`'s own protocol types ([`HttpRequest`], [`HttpResult`]),
//! so the gateway forwards exactly what the device's core asked for. Every request has an
//! `id`, because requests overlap (a GET, POSTs and an SSE stream can all be in flight).

#![no_std]

extern crate alloc;

pub mod sse;

use alloc::vec::Vec;

use crux_http::protocol::{HttpRequest, HttpResult};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub use sse::{SseRequest, SseResponse};

/// The gateway service.
pub const SERVICE_UUID: &str = "744aa621-b4b9-4751-91c0-a43b54bb1544";
/// [`SERVICE_UUID`] as a number, for advertising data.
pub const SERVICE_UUID_U128: u128 = 0x744a_a621_b4b9_4751_91c0_a43b_54bb_1544;
/// Device → gateway (notify).
pub const TX_UUID: &str = "ad229cac-4e64-4208-b76b-ed536a1e47c5";
/// Gateway → device (write without response).
pub const RX_UUID: &str = "cb4206ec-fe9c-4fea-ae65-2e7c5b56515b";

/// The chunk size the gateway writes with. Chrome does not expose the negotiated ATT
/// MTU, and 20 bytes (the minimum MTU of 23, less 3) always fits.
pub const GATEWAY_CHUNK: usize = 20;

/// The largest message either side accepts. A frame announcing more resets the
/// [`Reassembler`] instead of buffering it.
pub const MAX_MESSAGE: usize = 8 * 1024;

/// Identifies a request, and the response(s) to it, on the link.
pub type Id = u16;

/// Device → gateway.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ToGateway {
    /// Perform this HTTP request; answer with [`ToDevice::Http`].
    Http { id: Id, request: HttpRequest },
    /// Open this Server-Sent Events stream; answer with [`ToDevice::SseChunk`]s, then
    /// [`ToDevice::SseDone`].
    Sse { id: Id, request: SseRequest },
    /// The device no longer wants the answer to `id` (for a stream: close it).
    Cancel { id: Id },
}

/// Gateway → device.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ToDevice {
    Http {
        id: Id,
        result: HttpResult,
    },
    SseChunk {
        id: Id,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },
    SseDone {
        id: Id,
    },
}

/// Why a message could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A frame announced more than [`MAX_MESSAGE`] bytes.
    TooLong(usize),
    /// The bytes are not a valid postcard encoding of the message type.
    Decode,
}

/// Encode `message` as one frame: a `u16` little-endian length, then the postcard bytes.
///
/// # Panics
/// If the encoding is longer than [`MAX_MESSAGE`]. Use [`try_encode`] for messages whose
/// size is not under the sender's control (an HTTP response body).
#[must_use]
pub fn encode<T: Serialize>(message: &T) -> Vec<u8> {
    try_encode(message).expect("message too long for the link")
}

/// [`encode`], or `None` if the message is longer than [`MAX_MESSAGE`].
#[must_use]
pub fn try_encode<T: Serialize>(message: &T) -> Option<Vec<u8>> {
    let body = postcard::to_allocvec(message).ok()?;
    if body.len() > MAX_MESSAGE {
        return None;
    }
    let len = u16::try_from(body.len()).ok()?;

    let mut frame = Vec::with_capacity(2 + body.len());
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&body);
    Some(frame)
}

/// Cut a frame into pieces of at most `max` bytes, to write or notify one by one.
///
/// # Panics
/// If `max` is zero.
pub fn chunks(frame: &[u8], max: usize) -> impl Iterator<Item = &[u8]> {
    frame.chunks(max)
}

/// Decode one message from the bytes of a frame (as yielded by [`Reassembler::push`]).
///
/// # Errors
/// [`Error::Decode`] if the bytes are not a `T`.
pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    postcard::from_bytes(bytes).map_err(|_| Error::Decode)
}

/// Turns the chunks arriving on one characteristic back into messages.
///
/// It treats the chunks as a byte stream, so one chunk may finish a message and start
/// the next. Call [`Reassembler::reset`] when the link drops: a half-received frame from
/// the old connection must not be glued to the new one.
#[derive(Debug, Default)]
pub struct Reassembler {
    buf: Vec<u8>,
}

impl Reassembler {
    #[must_use]
    pub const fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn reset(&mut self) {
        self.buf.clear();
    }

    /// Add a chunk; return the body of every message it completes, in order.
    ///
    /// # Errors
    /// [`Error::TooLong`] if a frame announces more than [`MAX_MESSAGE`] bytes. The buffer
    /// is cleared, so the stream starts over with the next chunk.
    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<Vec<u8>>, Error> {
        self.buf.extend_from_slice(chunk);

        let mut messages = Vec::new();
        while let Some(&[lo, hi]) = self.buf.first_chunk::<2>() {
            let len = usize::from(u16::from_le_bytes([lo, hi]));
            if len > MAX_MESSAGE {
                self.buf.clear();
                return Err(Error::TooLong(len));
            }
            if self.buf.len() < 2 + len {
                break;
            }
            messages.push(self.buf[2..2 + len].to_vec());
            self.buf.drain(..2 + len);
        }
        Ok(messages)
    }
}

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec};

    use crux_http::{
        HttpError,
        protocol::{HttpRequest, HttpResponse, HttpResult},
    };

    use super::*;

    fn samples() -> Vec<(ToGateway, ToDevice)> {
        vec![
            (
                ToGateway::Http {
                    id: 1,
                    request: HttpRequest::post("https://crux-counter.fly.dev/inc")
                        .header("content-type", "application/json")
                        .body(r#"{"value":1}"#)
                        .build(),
                },
                ToDevice::Http {
                    id: 1,
                    result: HttpResult::Ok(
                        HttpResponse::ok()
                            .header("content-type", "application/json")
                            .body(r#"{"value":2,"updated_at":1672531200000}"#)
                            .build(),
                    ),
                },
            ),
            (
                ToGateway::Sse {
                    id: 2,
                    request: SseRequest {
                        url: "https://crux-counter.fly.dev/sse".to_string(),
                    },
                },
                ToDevice::SseChunk {
                    id: 2,
                    data: b"data: {\"value\":3}\n\n".to_vec(),
                },
            ),
            (
                ToGateway::Cancel { id: 2 },
                ToDevice::Http {
                    id: 3,
                    result: HttpResult::Err(HttpError::Io("offline".to_string())),
                },
            ),
            (
                ToGateway::Cancel { id: u16::MAX },
                ToDevice::SseDone { id: 2 },
            ),
        ]
    }

    /// Every message survives encode -> chunk -> reassemble -> decode, at every chunk size
    /// from 1 byte to larger than the frame.
    #[test]
    fn round_trip_at_every_chunk_size() {
        for (up, down) in samples() {
            let up_frame = encode(&up);
            let down_frame = encode(&down);
            for size in 1..=up_frame.len().max(down_frame.len()) + 1 {
                let mut rx = Reassembler::new();
                let mut got = Vec::new();
                for chunk in chunks(&up_frame, size) {
                    got.extend(rx.push(chunk).unwrap());
                }
                assert_eq!(got.len(), 1, "size {size}");
                assert_eq!(decode::<ToGateway>(&got[0]).unwrap(), up);

                let mut rx = Reassembler::new();
                let mut got = Vec::new();
                for chunk in chunks(&down_frame, size) {
                    got.extend(rx.push(chunk).unwrap());
                }
                assert_eq!(got.len(), 1, "size {size}");
                assert_eq!(decode::<ToDevice>(&got[0]).unwrap(), down);
            }
        }
    }

    /// Back-to-back messages, sliced without regard to where one ends.
    #[test]
    fn a_stream_of_messages_in_arbitrary_slices() {
        let messages: Vec<ToDevice> = samples().into_iter().map(|(_, down)| down).collect();
        let stream: Vec<u8> = messages.iter().flat_map(encode).collect();

        for size in [1, 3, 7, 20, 64, 244, stream.len()] {
            let mut rx = Reassembler::new();
            let mut got = Vec::new();
            for chunk in stream.chunks(size) {
                for body in rx.push(chunk).unwrap() {
                    got.push(decode::<ToDevice>(&body).unwrap());
                }
            }
            assert_eq!(got, messages, "size {size}");
        }
    }

    #[test]
    fn an_oversized_frame_resets_the_stream() {
        let mut rx = Reassembler::new();
        let too_long = u16::try_from(MAX_MESSAGE + 1).unwrap().to_le_bytes();
        assert_eq!(rx.push(&too_long), Err(Error::TooLong(MAX_MESSAGE + 1)));

        // The next frame is read from scratch.
        let frame = encode(&ToDevice::SseDone { id: 9 });
        let got = rx.push(&frame).unwrap();
        assert_eq!(decode::<ToDevice>(&got[0]), Ok(ToDevice::SseDone { id: 9 }));
    }

    #[test]
    fn reset_drops_a_half_received_frame() {
        let frame = encode(&ToDevice::SseDone { id: 1 });
        let mut rx = Reassembler::new();
        assert_eq!(
            rx.push(&frame[..frame.len() - 1]).unwrap(),
            [] as [alloc::vec::Vec<u8>; 0]
        );
        rx.reset();
        let got = rx.push(&encode(&ToDevice::SseDone { id: 2 })).unwrap();
        assert_eq!(decode::<ToDevice>(&got[0]), Ok(ToDevice::SseDone { id: 2 }));
    }

    #[test]
    fn a_message_over_the_limit_does_not_encode() {
        let big = ToDevice::SseChunk {
            id: 1,
            data: vec![0; MAX_MESSAGE],
        };
        assert_eq!(try_encode(&big), None);
    }

    #[test]
    fn garbage_does_not_decode() {
        assert_eq!(decode::<ToDevice>(&[0xff, 0xff, 0xff]), Err(Error::Decode));
    }

    /// The counter's messages are small: a POST is one notification at Chrome's usual
    /// MTU (85 bytes), and its answer is a handful of 20-byte writes.
    #[test]
    fn sizes() {
        let (up, down) = &samples()[0];
        assert_eq!(encode(up).len(), 85);
        assert_eq!(encode(down).len(), 77); // 4 writes of 20 bytes
    }
}
