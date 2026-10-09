//! The gateway's Bluetooth capability: connect to the device, and write to it.
//!
//! Custom to this app, so the Leptos shell implements both with Web Bluetooth.

use crux_core::macros::Operation;

/// Connect to a device offering `service`, and subscribe to notifications on `tx`.
///
/// The shell also gets ready to write to `rx`, and answers with a stream: `Connected`, a `Data`
/// per notification, and finally `Disconnected` (or `Failed`, if it never connected).
#[derive(Operation, Debug, Clone, PartialEq, Eq)]
#[operation(stream, output = BleEvent)]
pub struct BleConnect {
    pub service: String,
    pub tx: String,
    pub rx: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BleEvent {
    Connected {
        name: String,
    },
    /// One notification's value.
    Data(Vec<u8>),
    Disconnected,
    Failed(String),
}

/// Write `data` to the connected device's `rx` characteristic, in pieces of at most
/// `chunk` bytes, one after another.
#[derive(Operation, Debug, Clone, PartialEq, Eq)]
#[operation(request, output = WriteResult)]
pub struct BleWrite {
    pub data: Vec<u8>,
    pub chunk: usize,
}

/// The outcome of a [`BleWrite`]: an error message from Web Bluetooth, if it failed.
/// (An alias because `#[operation(output = …)]` cannot parse the comma in
/// `Result<(), String>`.)
pub type WriteResult = Result<(), String>;
