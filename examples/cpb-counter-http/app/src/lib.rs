//! spike(no_std): counter_http's Crux app, in a `no_std` crate, for the CPB.
//!
//! The HTTP code is counter_http's (`examples/counter-http/shared/src/app.rs`) unchanged:
//! `Http::get(..).expect_json().build().then_send(..)`, `Url::parse(..).join(..)`,
//! `crux_http::Result<crux_http::Response<Count>>`, `response.take_body()`. It compiles
//! against crux_http's no_std subset. What differs, and why:
//!
//! - `Count::updated_at` is epoch milliseconds, not a chrono `DateTime`: nothing here
//!   displays a date, and "pending" is just `updated_at.is_none()`.
//! - The view is ten NeoPixels and the red LED, not a string.
//! - `Set(Err(_))` sets an error flag instead of `panic!`: with `panic-halt`, a panic
//!   freezes the board until reset.
//! - New events: `Connected`/`Disconnected` (the BLE link to the gateway; on connect the
//!   app does what counter_http's shells do at start-up: `Get`, then `StartWatch`), and
//!   `Switch` (the slide switch picks the brightness).
//! - Button A is `Increment`, button B is `Decrement`.

#![no_std]

extern crate alloc;

pub mod sse;

use crux_core::{
    App, Command,
    macros::effect,
    render::{RenderOperation, render},
};
use crux_http::{command::Http, protocol::HttpRequest};
use serde::{Deserialize, Serialize};
use url::Url;

use cpb_protocol::SseRequest;
use sse::ServerSentEvents;

const API_URL: &str = "https://crux-counter.fly.dev";

/// Number of NeoPixels on the board.
pub const PIXELS: usize = 10;

#[derive(Default)]
pub struct Model {
    count: Count,
    connected: bool,
    error: bool,
    bright: bool,
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
}

// Plain `#[effect]`: `facet_typegen` would need crux_core's `bridge` (std).
#[effect]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
    ServerSentEvents(SseRequest),
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
            Event::StartWatch => {
                let base = Url::parse(API_URL).unwrap();
                let url = base.join("/sse").unwrap();
                ServerSentEvents::get(url).then_send(Event::Update)
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
        let colour = if model.count.value >= 0 {
            Rgb {
                r: 0,
                g: level,
                b: 0,
            }
        } else {
            Rgb {
                r: level,
                g: 0,
                b: 0,
            }
        };
        let lit = model.count.value.unsigned_abs().min(PIXELS);
        for pixel in pixels.iter_mut().take(lit) {
            *pixel = colour;
        }

        ViewModel {
            pixels,
            led_on: pending,
        }
    }
}

#[cfg(test)]
mod tests;
