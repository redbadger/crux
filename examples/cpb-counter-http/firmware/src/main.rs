//! spike(no_std): the firmware "shell" for counter_http's Crux core on the Circuit
//! Playground Bluefruit. The core's HTTP and SSE effects go over BLE to the Chrome
//! gateway (`../gateway`), which performs them.
//!
//! Pins (CircuitPython `ports/nordic/boards/circuitplayground_bluefruit`):
//!   button A    P1.02  (active high, needs pull-down)   -> Increment
//!   button B    P1.15  (active high, needs pull-down)   -> Decrement
//!   slide sw.   P1.06  (needs pull-up)                  -> brightness
//!   red LED     P1.14  (D13, active high)               -> count pending
//!   NeoPixels   P0.13  (10 x WS2812-style)              -> the count
//!   power ctl   P0.06  (low = NeoPixels and sensors powered)

#![no_std]
#![no_main]

extern crate alloc;

mod link;
mod neopixel;

use alloc::collections::BTreeMap;
use alloc::string::ToString;
use alloc::vec::Vec;

use crux_core::{Core, Request};
use crux_http::{
    HttpError,
    protocol::{HttpRequest, HttpResult},
};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_futures::select::{Either4, select4};
use embassy_nrf::gpio::{Input, Level, Output, OutputDrive, Pull};
use embassy_time::{Duration, Timer};
use embedded_alloc::LlffHeap as Heap;
use panic_halt as _;

use cpb_counter_http_app::{Counter, Effect, Event, ViewModel};
use cpb_protocol::{Id, SseRequest, SseResponse, ToDevice, ToGateway};
use link::{INCOMING, LinkEvent, OUTGOING};
use neopixel::{Frame, NeoPixels, PIXELS};

#[global_allocator]
static HEAP: Heap = Heap::empty();

/// Heap for Crux (boxed futures, channels, effect vectors), crux_http, serde_json and
/// postcard. The heap probe (32-bit) measures ~3.3 KB with the SSE stream open and
/// ~1.6 KB more per request in flight: 20 in flight peaked at 45 KB.
const HEAP_SIZE: usize = 64 * 1024;

/// More HTTP requests than this in flight are failed at once rather than sent, so a
/// button-masher cannot exhaust the heap (with `panic-halt`, that would freeze the board).
const MAX_IN_FLIGHT: usize = 16;

/// How long an input must settle after an edge before its level is believed.
const DEBOUNCE: Duration = Duration::from_millis(30);

/// An input that reports only settled changes of level, on press *and* release, so
/// contact bounce on either edge cannot produce extra events (as in cpb-counter).
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

struct Shell<'a> {
    core: Core<Counter>,
    led: Output<'a>,
    pixels: NeoPixels<'a>,
    /// The gateway is connected and subscribed.
    up: bool,
    next_id: Id,
    http: BTreeMap<Id, Request<HttpRequest>>,
    sse: BTreeMap<Id, Request<SseRequest>>,
}

impl Shell<'_> {
    fn event(&mut self, event: Event) {
        let effects = self.core.process_event(event);
        self.handle(effects);
    }

    fn handle(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::Render(_) => {
                    let view = self.core.view();
                    self.render(&view);
                }
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
                    let message = ToGateway::Sse {
                        id,
                        request: request.operation.clone(),
                    };
                    if self.up && OUTGOING.try_send(message).is_ok() {
                        self.sse.insert(id, request);
                    } else {
                        let effects = self
                            .core
                            .resolve(&mut request, SseResponse::Done)
                            .unwrap_or_default();
                        self.handle(effects);
                    }
                }
            }
        }
    }

    fn link(&mut self, event: LinkEvent) {
        match event {
            LinkEvent::Up => {
                self.up = true;
                self.event(Event::Connected);
            }
            LinkEvent::Advertising | LinkEvent::Down => self.down(),
            LinkEvent::Failed => {
                self.down();
                self.pixels.show(&[[24, 0, 0]; PIXELS]);
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
                match self.core.resolve(request, SseResponse::Chunk(data)) {
                    Ok(effects) => self.handle(effects),
                    Err(_) => {
                        // The core no longer wants this stream.
                        self.sse.remove(&id);
                        let _ = OUTGOING.try_send(ToGateway::Cancel { id });
                    }
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

    /// The gateway went away: fail what was in flight, end the streams, tell the core.
    fn down(&mut self) {
        if !self.up {
            return;
        }
        self.up = false;
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
        self.event(Event::Disconnected);
    }

    const fn id(&mut self) -> Id {
        self.next_id = self.next_id.wrapping_add(1);
        self.next_id
    }

    fn render(&mut self, view: &ViewModel) {
        self.led
            .set_level(if view.led_on { Level::High } else { Level::Low });
        let mut frame: Frame = [[0; 3]; PIXELS];
        for (out, pixel) in frame.iter_mut().zip(view.pixels.iter()) {
            *out = [pixel.r, pixel.g, pixel.b];
        }
        self.pixels.show(&frame);
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

    let p = embassy_nrf::init(Default::default());

    // Power the NeoPixels.
    let _power = Output::new(p.P0_06, Level::Low, OutputDrive::Standard);
    let led = Output::new(p.P1_14, Level::Low, OutputDrive::Standard);
    let mut button_a = Debounced::new(Input::new(p.P1_02, Pull::Down));
    let mut button_b = Debounced::new(Input::new(p.P1_15, Pull::Down));
    let mut switch = Debounced::new(Input::new(p.P1_06, Pull::Up));
    let pixels = NeoPixels::new(p.PWM0, p.P0_13);

    let mut shell = Shell {
        core: Core::new(),
        led,
        pixels,
        up: false,
        next_id: 0,
        http: BTreeMap::new(),
        sse: BTreeMap::new(),
    };
    // The slide switch's position (left, low, is bright); this also renders the
    // "waiting for the gateway" view.
    shell.event(Event::Switch(!switch.high));

    let Some(controller) = link::controller(spawner, radio!(p)) else {
        shell.pixels.show(&[[24, 0, 0]; PIXELS]);
        return;
    };

    join(link::run(controller), async {
        loop {
            match select4(
                INCOMING.receive(),
                button_a.changed(),
                button_b.changed(),
                switch.changed(),
            )
            .await
            {
                Either4::First(event) => shell.link(event),
                // Buttons are active high: count presses, ignore releases.
                Either4::Second(pressed) => {
                    if pressed && shell.up {
                        shell.event(Event::Increment);
                    }
                }
                Either4::Third(pressed) => {
                    if pressed && shell.up {
                        shell.event(Event::Decrement);
                    }
                }
                Either4::Fourth(high) => shell.event(Event::Switch(!high)),
            }
        }
    })
    .await;
}
