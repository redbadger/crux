//! `spike(no_std)`: a firmware shell for the counter-http example's core (`../shared`) on
//! the Circuit Playground Bluefruit. It calls `Core` directly; there is no FFI bridge.
//!
//! Like any Crux shell it turns input into events, handles the effects the core asks for,
//! and draws the view. The core is the same one the iOS, Android and web shells use; what
//! is particular to this board stays here:
//!   - the core's HTTP requests and Server-Sent Events go over BLE to the Chrome gateway
//!     (`examples_support/ble_gateway`), which performs them;
//!   - its timers (`crux_time`, which paces reopening the event stream) are resolved by
//!     their own task (`delay.rs`), whose follow-up effects come back through a queue the
//!     main loop selects on;
//!   - when the gateway connects, the shell sends `Get` and `StartWatch`, as the other
//!     shells do at start-up;
//!   - button A sends `Increment` and button B sends `Decrement`;
//!   - the view's `value` is drawn on the ring of `NeoPixels` as an odometer, dimmed while
//!     the count is waiting for the server; the view's `error`, and the shell's own link
//!     state, have pixel patterns of their own;
//!   - the slide switch picks the brightness, which is presentation, so the core never
//!     hears about it.
//!
//! Pins (`CircuitPython` `ports/nordic/boards/circuitplayground_bluefruit`):
//!   button A    P1.02  (active high, needs pull-down)   -> Increment
//!   button B    P1.15  (active high, needs pull-down)   -> Decrement
//!   slide sw.   P1.06  (needs pull-up)                  -> brightness
//!   red LED     P1.14  (D13, active high)               -> count pending
//!   `NeoPixels`   P0.13  (10 x WS2812-style)              -> the count
//!   power ctl   P0.06  (low = `NeoPixels` and sensors powered)

#![no_std]
#![no_main]
// Embassy's thread-mode executor is single-threaded: nothing here needs to be `Send`.
#![allow(clippy::future_not_send)]

extern crate alloc;

mod delay;
mod link;
mod neopixel;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::ToString;

use crux_core::Request;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_futures::select::{Either, Either4, select, select4};
use embassy_nrf::gpio::{Input, Level, Output, OutputDrive, Pull};
use embassy_time::{Duration, Timer};
use embedded_alloc::LlffHeap as Heap;
use panic_halt as _;

use ble_protocol::{Id, ToDevice, ToGateway};
use shared::http::{
    HttpError,
    protocol::{HttpRequest, HttpResult},
};
use shared::sse::{SseRequest, SseResponse};
use shared::{Core, Counter, Effect, Event, ViewModel};

use delay::{EFFECTS, TIMERS, TimeRequest};
use link::{INCOMING, LinkEvent, OUTGOING};
use neopixel::{Frame, NeoPixels, PIXELS};

#[global_allocator]
static HEAP: Heap = Heap::empty();

/// Heap for Crux (boxed futures, channels, effect vectors), `crux_http`, `serde_json` and
/// postcard. The heap probe (32-bit) measures ~3.3 KB with the SSE stream open and
/// ~1.6 KB more per request in flight: 20 in flight peaked at 45 KB.
const HEAP_SIZE: usize = 64 * 1024;

/// More HTTP requests than this in flight are failed at once rather than sent, so a
/// button-masher cannot exhaust the heap (with `panic-halt`, that would freeze the board).
const MAX_IN_FLIGHT: usize = 16;

/// Pixel brightness (0–255) for each position of the slide switch.
const BRIGHT: u8 = 40;
const DIM: u8 = 6;

/// How long an input must settle after an edge before its level is believed.
const DEBOUNCE: Duration = Duration::from_millis(30);

/// An input that reports only settled changes of level, on press *and* release, so
/// contact bounce on either edge cannot produce extra events.
struct Debounced<'a> {
    input: Input<'a>,
    high: bool,
}

impl<'a> Debounced<'a> {
    fn new(input: Input<'a>) -> Self {
        let high = input.is_high();
        Self { input, high }
    }

    /// Waits for the input to settle at the opposite level, records it and returns it
    /// (`true` for high). Cancel-safe: `high` changes only on return.
    async fn changed(&mut self) -> bool {
        loop {
            if self.high {
                self.input.wait_for_low().await;
            } else {
                self.input.wait_for_high().await;
            }
            Timer::after(DEBOUNCE).await;
            let high = self.input.is_high();
            if high != self.high {
                self.high = high;
                return high;
            }
        }
    }
}

// Three independent facts about the shell (link, radio, switch), not a state machine.
#[allow(clippy::struct_excessive_bools)]
struct Shell<'a> {
    core: &'static Core<Counter>,
    led: Output<'a>,
    pixels: NeoPixels<'a>,
    /// The gateway is connected and subscribed.
    up: bool,
    /// The radio stack failed; the link is gone until reset.
    failed: bool,
    /// Shell-local state: the slide switch's brightness.
    bright: bool,
    next_id: Id,
    http: BTreeMap<Id, Request<HttpRequest>>,
    sse: BTreeMap<Id, Request<SseRequest>>,
}

impl Shell<'_> {
    /// Sends an event to the core and handles the effects it asks for.
    fn process_event(&mut self, event: Event) {
        let effects = self.core.process_event(event);
        self.handle(effects);
    }

    fn handle(&mut self, effects: impl IntoIterator<Item = Effect>) {
        for effect in effects {
            match effect {
                Effect::Render(_) => self.render(),
                Effect::Http(mut request) => {
                    let id = self.id();
                    let message = ToGateway::Http {
                        id,
                        request: request.operation.clone(),
                    };
                    let refusal = if !self.up {
                        Some("gateway not connected")
                    } else if self.http.len() >= MAX_IN_FLIGHT {
                        Some("too many requests in flight")
                    } else if OUTGOING.try_send(message).is_err() {
                        Some("link queue full")
                    } else {
                        None
                    };
                    if let Some(reason) = refusal {
                        let effects = self
                            .core
                            .resolve(
                                &mut request,
                                HttpResult::Err(HttpError::Io(reason.to_string())),
                            )
                            .unwrap_or_default();
                        self.handle(effects);
                    } else {
                        self.http.insert(id, request);
                    }
                }
                Effect::ServerSentEvents(mut request) => {
                    let id = self.id();
                    // The core's SSE capability is its own; the link carries a wire copy.
                    let message = ToGateway::Sse {
                        id,
                        request: ble_protocol::SseRequest {
                            url: request.operation.url.clone(),
                        },
                    };
                    if self.up && OUTGOING.try_send(message).is_ok() {
                        self.sse.insert(id, request);
                    } else {
                        // The stream ends at once; the core reopens it after a wait.
                        let effects = self
                            .core
                            .resolve(&mut request, SseResponse::Done)
                            .unwrap_or_default();
                        self.handle(effects);
                    }
                }
                Effect::TimeNotifyAfter(request) => TIMERS.push(TimeRequest::NotifyAfter(request)),
                Effect::TimeClear(request) => TIMERS.push(TimeRequest::Clear(request)),
            }
        }
    }

    fn link(&mut self, event: LinkEvent) {
        match event {
            LinkEvent::Up => {
                self.up = true;
                self.render();
                // What the other shells do at start-up.
                self.process_event(Event::Get);
                self.process_event(Event::StartWatch);
            }
            LinkEvent::Advertising | LinkEvent::Down => self.down(),
            LinkEvent::Failed => {
                self.down();
                self.failed = true;
                self.render();
            }
            LinkEvent::Message(ToDevice::Http { id, result }) => {
                if let Some(mut request) = self.http.remove(&id) {
                    let effects = self.core.resolve(&mut request, result).unwrap_or_default();
                    self.handle(effects);
                }
            }
            LinkEvent::Message(ToDevice::SseChunk { id, data }) => {
                let Some(request) = self.sse.get_mut(&id) else {
                    return;
                };
                if let Ok(effects) = self.core.resolve(request, SseResponse::Chunk(data)) {
                    self.handle(effects);
                } else {
                    // The core no longer wants this stream.
                    self.sse.remove(&id);
                    let _ = OUTGOING.try_send(ToGateway::Cancel { id });
                }
            }
            LinkEvent::Message(ToDevice::SseDone { id }) => {
                if let Some(mut request) = self.sse.remove(&id) {
                    let effects = self
                        .core
                        .resolve(&mut request, SseResponse::Done)
                        .unwrap_or_default();
                    self.handle(effects);
                }
            }
        }
    }

    /// The gateway went away: fail what was in flight and end the streams. The core
    /// reopens its stream after a wait, which fails again until the gateway is back.
    fn down(&mut self) {
        if !self.up {
            return;
        }
        self.up = false;
        self.render();
        for (_, mut request) in core::mem::take(&mut self.http) {
            let effects = self
                .core
                .resolve(
                    &mut request,
                    HttpResult::Err(HttpError::Io("ble disconnected".to_string())),
                )
                .unwrap_or_default();
            self.handle(effects);
        }
        for (_, mut request) in core::mem::take(&mut self.sse) {
            let effects = self
                .core
                .resolve(&mut request, SseResponse::Done)
                .unwrap_or_default();
            self.handle(effects);
        }
    }

    const fn id(&mut self) -> Id {
        self.next_id = self.next_id.wrapping_add(1);
        self.next_id
    }

    /// Draws the core's current view, with the shell's own state, on the ring and D13.
    fn render(&mut self) {
        let view = self.core.view();
        let (frame, pending) = frame(&view, self.up, self.failed, self.bright);
        self.led
            .set_level(if pending { Level::High } else { Level::Low });
        self.pixels.show(&frame);
    }
}

/// The pixels for a view, and whether D13 should be lit (the count is waiting for the
/// server):
/// - all red: the radio stack failed;
/// - one dim blue pixel: waiting for the gateway;
/// - alternating red and blue: the last request failed (`view.error`);
/// - otherwise the count, as an odometer: the core's count is unbounded, so each lap of
///   ten fills in a new colour over the last lap's. 10 is ten green, 11 is one cyan over
///   nine green, 20 is ten cyan. A count waiting for the server is drawn at a third of the
///   brightness.
fn frame(view: &ViewModel, up: bool, failed: bool, bright: bool) -> (Frame, bool) {
    let mut frame: Frame = [[0; 3]; PIXELS];
    let level = if bright { BRIGHT } else { DIM };

    if failed {
        return ([[24, 0, 0]; PIXELS], false);
    }

    if !up {
        frame[0] = [0, 0, DIM];
        return (frame, false);
    }

    if view.error.is_some() {
        for (i, pixel) in frame.iter_mut().enumerate() {
            *pixel = if i % 2 == 0 {
                [level, 0, 0]
            } else {
                [0, 0, level]
            };
        }
        return (frame, false);
    }

    let pending = !view.confirmed;
    let level = if pending { (level / 3).max(2) } else { level };
    let positive = view.value >= 0;
    let n = view.value.unsigned_abs() as usize;
    if n > 0 {
        let lap = (n - 1) / PIXELS;
        let units = (n - 1) % PIXELS + 1;
        for (i, pixel) in frame.iter_mut().enumerate() {
            if i < units {
                *pixel = lap_colour(positive, lap, level);
            } else if lap > 0 {
                *pixel = lap_colour(positive, lap - 1, level);
            }
        }
    }
    (frame, pending)
}

/// The colour of lap `lap` (0 for 1..=10, 1 for 11..=20, ...): green, cyan, blue for
/// positive counts, and red, orange, magenta for negative ones, round and round.
const fn lap_colour(positive: bool, lap: usize, level: u8) -> [u8; 3] {
    match (positive, lap % 3) {
        (true, 0) => [0, level, 0],
        (true, 1) => [0, level, level],
        (true, _) => [0, 0, level],
        (false, 0) => [level, 0, 0],
        (false, 1) => [level, level / 2, 0],
        (false, _) => [level, 0, level],
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    {
        use core::mem::MaybeUninit;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        #[allow(static_mut_refs)]
        unsafe {
            HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE);
        }
    }

    let p = embassy_nrf::init(embassy_nrf::config::Config::default());

    // Power the NeoPixels.
    let _power = Output::new(p.P0_06, Level::Low, OutputDrive::Standard);
    let led = Output::new(p.P1_14, Level::Low, OutputDrive::Standard);
    let mut button_a = Debounced::new(Input::new(p.P1_02, Pull::Down));
    let mut button_b = Debounced::new(Input::new(p.P1_15, Pull::Down));
    let mut switch = Debounced::new(Input::new(p.P1_06, Pull::Up));
    let pixels = NeoPixels::new(p.PWM0, p.P0_13);

    // Shared by the main loop and the timer task, for the life of the firmware.
    let core: &'static Core<Counter> = Box::leak(Box::new(Core::new()));
    let Ok(token) = delay::timers(core) else {
        panic!("timer task already spawned");
    };
    spawner.spawn(token);

    let mut shell = Shell {
        core,
        led,
        pixels,
        up: false,
        failed: false,
        // The switch reads low in its left position: bright.
        bright: !switch.high,
        next_id: 0,
        http: BTreeMap::new(),
        sse: BTreeMap::new(),
    };
    // Draw the "waiting for the gateway" view. No event is needed to read the core's view.
    shell.render();

    let Some(controller) = link::controller(spawner, radio!(p)) else {
        shell.failed = true;
        shell.render();
        return;
    };

    join(link::run(controller), async {
        loop {
            let inputs = select4(
                INCOMING.receive(),
                button_a.changed(),
                button_b.changed(),
                switch.changed(),
            );
            let event = match select(inputs, EFFECTS.wait()).await {
                Either::First(event) => event,
                Either::Second(()) => {
                    shell.handle(EFFECTS.take());
                    continue;
                }
            };
            match event {
                Either4::First(event) => shell.link(event),
                // Buttons are active high: count presses, ignore releases. Presses while
                // the gateway is away are ignored rather than failed.
                Either4::Second(pressed) => {
                    if pressed && shell.up {
                        shell.process_event(Event::Increment);
                    }
                }
                Either4::Third(pressed) => {
                    if pressed && shell.up {
                        shell.process_event(Event::Decrement);
                    }
                }
                // Brightness is the shell's own business: redraw the same view.
                Either4::Fourth(high) => {
                    shell.bright = !high;
                    shell.render();
                }
            }
        }
    })
    .await;
}
