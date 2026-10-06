//! `spike(no_std)`: `counter_http`'s Crux app, in a `no_std` crate, for the CPB.
//!
//! The HTTP code is `counter_http`'s (`examples/counter-http/shared/src/app.rs`) unchanged:
//! `Http::get(..).expect_json().build().then_send(..)`, `Url::parse(..).join(..)`,
//! `crux_http::Result<crux_http::Response<Count>>`, `response.take_body()`. It compiles
//! against `crux_http`'s `no_std` subset. What differs, and why:
//!
//! - `Count::updated_at` is epoch milliseconds, not a chrono `DateTime`: nothing here
//!   displays a date, and "pending" is just `updated_at.is_none()`.
//! - The view is ten `NeoPixels` and the red LED, not a string.
//! - `Set(Err(_))` sets an error flag instead of `panic!`: with `panic-halt`, a panic
//!   freezes the board until reset.
//! - New events: `Connected`/`Disconnected` (the BLE link to the gateway; on connect the
//!   app does what `counter_http`'s shells do at start-up: `Get`, then `StartWatch`), and
//!   `Switch` (the slide switch picks the brightness).
//! - Button A is `Increment`, button B is `Decrement`.
//! - When the SSE stream ends (the gateway's fetch died with the network, say), the app
//!   resubscribes after a back-off (1 s, doubling to 30 s), using a `Delay` effect. A browser
//!   user of `counter_http` reloads the page; the board has nobody to do that.

#![no_std]

extern crate alloc;

pub mod sse;

use crux_core::{
    App, Command,
    macros::{Operation, effect},
    render::{RenderOperation, render},
};
use crux_http::{command::Http, protocol::HttpRequest};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use url::Url;

use cpb_protocol::SseRequest;
use sse::ServerSentEvents;

const API_URL: &str = "https://crux-counter.fly.dev";

/// Number of `NeoPixels` on the board.
pub const PIXELS: usize = 10;

/// The first wait before reopening a stream that ended, and the longest.
const WATCH_BACKOFF_MIN: u32 = 1_000;
const WATCH_BACKOFF_MAX: u32 = 30_000;

// Four independent facts (link, last request, brightness, stream open), not a state machine.
#[allow(clippy::struct_excessive_bools)]
pub struct Model {
    count: Count,
    connected: bool,
    error: bool,
    bright: bool,
    /// An SSE stream is open (or being opened).
    watching: bool,
    /// Identifies the current stream, so a stale end or retry timer is ignored.
    watch: u32,
    /// The wait before the next resubscribe.
    backoff: u32,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            count: Count::default(),
            connected: false,
            error: false,
            bright: false,
            watching: false,
            watch: 0,
            backoff: WATCH_BACKOFF_MIN,
        }
    }
}

/// Wait this long, then answer with `()` (as in cpb-counter). The firmware shell resolves
/// it with `embassy_time`.
#[derive(Operation, Debug, Clone, PartialEq, Eq)]
#[operation(request, output = ())]
pub struct Delay {
    pub millis: u32,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct Count {
    value: isize,
    /// Epoch milliseconds of the server's last change; `None` while an optimistic update
    /// is waiting for the server.
    updated_at: Option<i64>,
}

/// One RGB colour per pixel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ViewModel {
    pub pixels: [Rgb; PIXELS],
    /// The red LED: lit while the count is pending (not yet confirmed by the server).
    pub led_on: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    // events from the shell
    Connected,
    Disconnected,
    Get,
    Increment,
    Decrement,
    StartWatch,
    /// The slide switch moved; `true` is the left position (bright).
    Switch(bool),

    // events local to the core
    Set(crux_http::Result<crux_http::Response<Count>>),
    Update(Count),
    /// The SSE stream with this id ended.
    WatchEnded(u32),
    /// The back-off after stream `id` ended is over.
    Rewatch(u32),
}

// Plain `#[effect]`: `facet_typegen` would need crux_core's `bridge` (std).
#[effect]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
    ServerSentEvents(SseRequest),
    Delay(Delay),
}

#[derive(Default)]
pub struct Counter;

impl App for Counter {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::Connected => {
                model.connected = true;
                // Requests failed by the disconnect are history; the fresh GET decides.
                model.error = false;
                render()
                    .and(Command::event(Event::Get))
                    .and(Command::event(Event::StartWatch))
            }
            Event::Disconnected => {
                model.connected = false;
                // The shell ends the stream; forget it, and any retry timer for it.
                model.watching = false;
                model.watch = model.watch.wrapping_add(1);
                model.backoff = WATCH_BACKOFF_MIN;
                render()
            }
            Event::Switch(bright) => {
                model.bright = bright;
                render()
            }
            Event::Get => Http::get(API_URL)
                .expect_json()
                .build()
                .then_send(Event::Set),
            Event::Set(Ok(mut response)) => {
                let count = response.take_body().unwrap();
                Command::event(Event::Update(count))
            }
            Event::Set(Err(_)) => {
                model.error = true;
                render()
            }
            Event::Update(count) => {
                model.count = count;
                model.error = false;
                // The server is reachable again.
                model.backoff = WATCH_BACKOFF_MIN;
                render()
            }
            Event::Increment => {
                // optimistic update
                model.count = Count {
                    value: model.count.value + 1,
                    updated_at: None,
                };

                let call_api = {
                    let base = Url::parse(API_URL).unwrap();
                    let url = base.join("/inc").unwrap();
                    Http::post(url).expect_json().build().then_send(Event::Set)
                };

                render().and(call_api)
            }
            Event::Decrement => {
                // optimistic update
                model.count = Count {
                    value: model.count.value - 1,
                    updated_at: None,
                };

                let call_api = {
                    let base = Url::parse(API_URL).unwrap();
                    let url = base.join("/dec").unwrap();
                    Http::post(url).expect_json().build().then_send(Event::Set)
                };

                render().and(call_api)
            }
            Event::StartWatch => watch(model),
            Event::WatchEnded(id) => {
                if id != model.watch {
                    return Command::done();
                }
                model.watching = false;
                if !model.connected {
                    return Command::done();
                }
                let millis = model.backoff;
                model.backoff = (model.backoff * 2).min(WATCH_BACKOFF_MAX);
                Command::request_from_shell(Delay { millis })
                    .then_send(move |()| Event::Rewatch(id))
            }
            Event::Rewatch(id) => {
                if id == model.watch {
                    watch(model)
                } else {
                    Command::done()
                }
            }
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        let mut pixels = [Rgb::default(); PIXELS];
        let level: u8 = if model.bright { 40 } else { 6 };

        if !model.connected {
            // Waiting for the gateway: one dim blue pixel.
            pixels[0] = Rgb { r: 0, g: 0, b: 6 };
            return ViewModel {
                pixels,
                led_on: false,
            };
        }

        if model.error {
            // The last request failed: alternate red and blue.
            for (i, pixel) in pixels.iter_mut().enumerate() {
                *pixel = if i % 2 == 0 {
                    Rgb {
                        r: level,
                        g: 0,
                        b: 0,
                    }
                } else {
                    Rgb {
                        r: 0,
                        g: 0,
                        b: level,
                    }
                };
            }
            return ViewModel {
                pixels,
                led_on: false,
            };
        }

        let pending = model.count.updated_at.is_none();
        // Pending values are shown at a third of the brightness (and D13 is lit).
        let level = if pending { (level / 3).max(2) } else { level };
        // The server's count is unbounded, so the ring is an odometer: each lap of ten
        // fills in a new colour over the last lap's. 10 is ten green, 11 is one cyan over
        // nine green, 20 is ten cyan.
        let positive = model.count.value >= 0;
        let n = model.count.value.unsigned_abs();
        if n > 0 {
            let lap = (n - 1) / PIXELS;
            let units = (n - 1) % PIXELS + 1;
            for (i, pixel) in pixels.iter_mut().enumerate() {
                if i < units {
                    *pixel = lap_colour(positive, lap, level);
                } else if lap > 0 {
                    *pixel = lap_colour(positive, lap - 1, level);
                }
            }
        }

        ViewModel {
            pixels,
            led_on: pending,
        }
    }
}

/// The colour of lap `lap` (0 for 1..=10, 1 for 11..=20, ...): green, cyan, blue for
/// positive counts, and red, orange, magenta for negative ones, round and round.
const fn lap_colour(positive: bool, lap: usize, level: u8) -> Rgb {
    let (r, g, b) = match (positive, lap % 3) {
        (true, 0) => (0, level, 0),
        (true, 1) => (0, level, level),
        (true, _) => (0, 0, level),
        (false, 0) => (level, 0, 0),
        (false, 1) => (level, level / 2, 0),
        (false, _) => (level, 0, level),
    };
    Rgb { r, g, b }
}

/// Open the SSE stream, as `counter_http`'s `StartWatch` does, and also report when it ends.
fn watch(model: &mut Model) -> Command<Effect, Event> {
    if !model.connected || model.watching {
        return Command::done();
    }
    model.watching = true;
    model.watch = model.watch.wrapping_add(1);
    let id = model.watch;

    let base = Url::parse(API_URL).unwrap();
    let url = base.join("/sse").unwrap();
    Command::new(move |ctx| async move {
        let mut updates = ServerSentEvents::get(url).into_stream(ctx.clone());
        while let Some(count) = updates.next().await {
            ctx.send_event(Event::Update(count));
        }
        ctx.send_event(Event::WatchEnded(id));
    })
}

#[cfg(test)]
mod tests;
