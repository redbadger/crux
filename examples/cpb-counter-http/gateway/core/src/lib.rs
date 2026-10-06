//! spike: the gateway between the CPB and the internet, as a Crux app.
//!
//! The device (a BLE peripheral) sends [`ToGateway`] messages over its `tx`
//! characteristic. This core reassembles them, turns each request into an effect of its
//! own (`Http`, `ServerSentEvents`) for the shell to perform, and sends every answer back
//! as a [`ToDevice`] message over `rx`.
//!
//! It knows nothing about the counter: it forwards whatever `HttpRequest` the device's
//! core built, so it is a generic HTTP and SSE proxy for one BLE device.
//!
//! Writes go one at a time ([`Model::outbox`]): Chrome runs one GATT operation at a time
//! and rejects a second one started while the first is in progress.

pub mod ble;

use std::collections::{BTreeMap, VecDeque};

use cpb_protocol::{
    GATEWAY_CHUNK, Id, RX_UUID, Reassembler, SERVICE_UUID, SseRequest, SseResponse, TX_UUID,
    ToDevice, ToGateway, decode, encode,
};
use crux_core::{
    App, Command,
    command::AbortHandle,
    macros::effect,
    render::{RenderOperation, render},
};
use crux_http::protocol::{HttpRequest, HttpResult};

pub use ble::{BleConnect, BleEvent, BleWrite};

/// How many log lines the view keeps.
const LOG_LINES: usize = 200;

#[derive(Debug)]
pub enum Event {
    /// The user asked to connect to the device.
    Connect,

    // events local to the core
    Ble(BleEvent),
    Written(Result<(), String>),
    HttpDone {
        id: Id,
        result: HttpResult,
    },
    Sse {
        id: Id,
        response: SseResponse,
    },
}

#[effect]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    BleConnect(BleConnect),
    BleWrite(BleWrite),
    Http(HttpRequest),
    ServerSentEvents(SseRequest),
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Link {
    #[default]
    Disconnected,
    Connecting,
    Connected {
        name: String,
    },
}

#[derive(Default)]
pub struct Model {
    link: Link,
    rx: Reassembler,
    /// Frames waiting to be written, oldest first.
    outbox: VecDeque<Vec<u8>>,
    /// A `BleWrite` is in progress.
    writing: bool,
    /// Open SSE streams, by request id.
    streams: BTreeMap<Id, AbortHandle>,
    log: VecDeque<String>,
    stats: Stats,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub requests: u32,
    pub responses: u32,
    pub sse_chunks: u32,
    pub bytes_in: u32,
    pub bytes_out: u32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ViewModel {
    pub status: String,
    pub can_connect: bool,
    pub open_streams: usize,
    pub stats: Stats,
    /// Newest first.
    pub log: Vec<String>,
}

#[derive(Default)]
pub struct Gateway;

impl App for Gateway {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::Connect => {
                if model.link != Link::Disconnected {
                    return Command::done();
                }
                model.link = Link::Connecting;
                let connect = Command::stream_from_shell(BleConnect {
                    service: SERVICE_UUID.to_string(),
                    tx: TX_UUID.to_string(),
                    rx: RX_UUID.to_string(),
                })
                .then_send(Event::Ble);
                render().and(connect)
            }
            Event::Ble(BleEvent::Connected { name }) => {
                log(model, format!("connected to {name}"));
                model.link = Link::Connected { name };
                model.rx.reset();
                model.outbox.clear();
                model.writing = false;
                render()
            }
            Event::Ble(BleEvent::Data(bytes)) => {
                model.stats.bytes_in += len_u32(&bytes);
                let messages = match model.rx.push(&bytes) {
                    Ok(messages) => messages,
                    Err(error) => {
                        log(model, format!("bad frame from the device: {error:?}"));
                        return render();
                    }
                };
                let mut commands = vec![render()];
                for body in messages {
                    match decode::<ToGateway>(&body) {
                        Ok(message) => commands.push(forward(model, message)),
                        Err(error) => log(model, format!("bad message from the device: {error:?}")),
                    }
                }
                Command::all(commands)
            }
            Event::Ble(BleEvent::Disconnected) => {
                log(model, "disconnected".to_string());
                disconnected(model);
                render()
            }
            Event::Ble(BleEvent::Failed(error)) => {
                log(model, format!("bluetooth: {error}"));
                disconnected(model);
                render()
            }
            Event::Written(result) => {
                model.writing = false;
                if let Err(error) = result {
                    log(model, format!("write failed: {error}"));
                }
                render().and(flush(model))
            }
            Event::HttpDone { id, result } => {
                model.stats.responses += 1;
                match &result {
                    HttpResult::Ok(response) => log(
                        model,
                        format!("#{id} ← {} ({} B)", response.status, response.body.len()),
                    ),
                    HttpResult::Err(error) => log(model, format!("#{id} ← error: {error}")),
                }
                render().and(send(model, &ToDevice::Http { id, result }))
            }
            Event::Sse { id, response } => match response {
                SseResponse::Chunk(data) => {
                    model.stats.sse_chunks += 1;
                    log(model, format!("#{id} ← SSE chunk ({} B)", data.len()));
                    render().and(send(model, &ToDevice::SseChunk { id, data }))
                }
                SseResponse::Done => {
                    model.streams.remove(&id);
                    log(model, format!("#{id} ← SSE closed by the server"));
                    render().and(send(model, &ToDevice::SseDone { id }))
                }
            },
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        let status = match &model.link {
            Link::Disconnected => "Disconnected".to_string(),
            Link::Connecting => "Connecting…".to_string(),
            Link::Connected { name } => format!("Connected to {name}"),
        };
        ViewModel {
            status,
            can_connect: model.link == Link::Disconnected,
            open_streams: model.streams.len(),
            stats: model.stats,
            log: model.log.iter().rev().cloned().collect(),
        }
    }
}

/// Turn one message from the device into the command that performs it.
fn forward(model: &mut Model, message: ToGateway) -> Command<Effect, Event> {
    match message {
        ToGateway::Http { id, request } => {
            model.stats.requests += 1;
            log(model, format!("#{id} → {} {}", request.method, request.url));
            Command::request_from_shell(request)
                .then_send(move |result| Event::HttpDone { id, result })
        }
        ToGateway::Sse { id, request } => {
            model.stats.requests += 1;
            log(model, format!("#{id} → SSE {}", request.url));
            if let Some(old) = model.streams.remove(&id) {
                old.abort();
            }
            let stream = Command::stream_from_shell(request)
                .then_send(move |response| Event::Sse { id, response });
            model.streams.insert(id, stream.abort_handle());
            stream
        }
        ToGateway::Cancel { id } => {
            if let Some(stream) = model.streams.remove(&id) {
                stream.abort();
                log(model, format!("#{id} cancelled"));
            }
            Command::done()
        }
    }
}

/// Queue a message for the device, if it is still there.
fn send(model: &mut Model, message: &ToDevice) -> Command<Effect, Event> {
    if !matches!(model.link, Link::Connected { .. }) {
        // An answer that arrives after the link dropped has nobody to go to; the device
        // failed its requests when it saw the disconnect.
        return Command::done();
    }
    model.outbox.push_back(encode(message));
    flush(model)
}

/// Start writing the oldest queued frame, unless a write is already in progress.
fn flush(model: &mut Model) -> Command<Effect, Event> {
    if model.writing {
        return Command::done();
    }
    let Some(frame) = model.outbox.pop_front() else {
        return Command::done();
    };
    model.writing = true;
    model.stats.bytes_out += len_u32(&frame);
    Command::request_from_shell(BleWrite {
        data: frame,
        chunk: GATEWAY_CHUNK,
    })
    .then_send(Event::Written)
}

fn disconnected(model: &mut Model) {
    model.link = Link::Disconnected;
    model.rx.reset();
    model.outbox.clear();
    model.writing = false;
    // The device reopens its stream when it reconnects; don't leave this one forwarding
    // into the void (or, after a reconnect, duplicating the new one).
    for (_, stream) in std::mem::take(&mut model.streams) {
        stream.abort();
    }
}

fn log(model: &mut Model, line: String) {
    if model.log.len() == LOG_LINES {
        model.log.pop_front();
    }
    model.log.push_back(line);
}

fn len_u32(bytes: &[u8]) -> u32 {
    u32::try_from(bytes.len()).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests;
