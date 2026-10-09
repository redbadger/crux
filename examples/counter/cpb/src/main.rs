//! `spike(no_std)`: a firmware shell for the counter's core (`../shared`) on the
//! Circuit Playground Bluefruit. It calls `Core` directly; there is no FFI bridge.
//!
//! Like any Crux shell it turns input into events, handles the effects the core
//! asks for, and draws the view. The core is the same one the iOS, Android and
//! web shells use; what is particular to this board stays here:
//!   - button A sends `Increment` and button B sends `Decrement`;
//!   - the view's `value` is drawn on the ring of `NeoPixels` as an odometer:
//!     the core's count is unbounded, so each lap of ten fills a new colour over
//!     the last (green, cyan, blue counting up; red, orange, magenta counting
//!     down);
//!   - the slide switch picks the brightness, and the red LED flashes on each
//!     press. Both are presentation, so the core never hears about them.
//!
//! Pins (`CircuitPython` `ports/nordic/boards/circuitplayground_bluefruit`):
//!   button A    P1.02  (active high, needs pull-down)
//!   button B    P1.15  (active high, needs pull-down)
//!   slide sw.   P1.06  (needs pull-up)
//!   red LED     P1.14  (D13, active high)
//!   `NeoPixels`   P0.13  (10 x WS2812-style)
//!   power ctl   P0.06  (low = `NeoPixels` and sensors powered)

#![no_std]
#![no_main]
// Embassy's thread-mode executor is single-threaded: nothing here needs to be `Send`.
#![allow(clippy::future_not_send)]

extern crate alloc;

use alloc::vec::Vec;

use embassy_executor::Spawner;
use embassy_futures::select::{Either4, select4};
use embassy_nrf::gpio::{Input, Level, Output, OutputDrive, Pull};
use embassy_nrf::pwm::{
    Config as PwmConfig, Prescaler, SequenceConfig, SequenceLoad, SequencePwm, SingleSequenceMode,
    SingleSequencer,
};
use embassy_time::{Duration, Instant, Timer};
use embedded_alloc::LlffHeap as Heap;
use panic_halt as _;

use shared::{Core, Counter, Effect, Event};

// ANCHOR: allocator
#[global_allocator]
static HEAP: Heap = Heap::empty();

/// Heap for Crux: the core's command and effect queues. Every event here
/// finishes with a single `Render`, so nothing stays in flight between presses,
/// and the host heap probe (heap-probe/, 64-bit pointers) peaks at about 3 KB.
/// 32 KB is generous; RAM is not scarce here.
const HEAP_SIZE: usize = 32 * 1024;
// ANCHOR_END: allocator

/// Number of `NeoPixels` on the board: one lap of the odometer.
const PIXELS: usize = 10;

/// Pixel brightness (0–255) for each position of the slide switch.
const BRIGHT: u8 = 40;
const DIM: u8 = 6;

/// How long the red LED stays on after a press.
const FLASH: Duration = Duration::from_millis(120);

// WS2812 timing with PWM at 16 MHz, 20 ticks per bit (1.25 us). The high bit
// inverts polarity, so the line starts high.
// WS2812 timing in PWM ticks; the duty reads best in decimal next to the polarity bit.
#[allow(clippy::decimal_bitwise_operands)]
const T1H: u16 = 0x8000 | 13;
// WS2812 timing in PWM ticks; the duty reads best in decimal next to the polarity bit.
#[allow(clippy::decimal_bitwise_operands)]
const T0H: u16 = 0x8000 | 7;
const RES: u16 = 0x8000;
const WORDS: usize = PIXELS * 24 + 1;

/// How long an input must settle after an edge before its level is believed.
const DEBOUNCE: Duration = Duration::from_millis(30);

type Pwm<'a> = SequencePwm<'a>;

/// One `NeoPixel`'s colour.
#[derive(Clone, Copy, Default)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

// ANCHOR: shell
struct Shell<'a> {
    core: Core<Counter>,
    pwm: Pwm<'a>,
    words: [u16; WORDS],
    /// Shell-local state: the slide switch's brightness.
    bright: bool,
    led: Output<'a>,
    /// Shell-local state: when the red LED's flash ends, if one is showing.
    led_off_at: Option<Instant>,
}
// ANCHOR_END: shell

/// An input that reports only settled changes of level, on press *and* release,
/// so contact bounce on either edge cannot produce extra events.
struct Debounced<'a> {
    input: Input<'a>,
    high: bool,
}

impl<'a> Debounced<'a> {
    fn new(input: Input<'a>) -> Self {
        let high = input.is_high();
        Self { input, high }
    }

    /// Waits for the input to settle at the opposite level, records it and
    /// returns it (`true` for high). Cancel-safe: `high` changes only on return,
    /// so a future dropped by `select4` starts over cleanly.
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

impl Shell<'_> {
    // ANCHOR: process_event
    /// Sends an event to the core and handles the effects it asks for.
    fn process_event(&mut self, event: Event) {
        let effects = self.core.process_event(event);
        self.handle(effects);
    }

    fn handle(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::Render(_) => self.render(),
            }
        }
    }
    // ANCHOR_END: process_event

    /// Lights the red LED for `FLASH`. The main loop turns it off again.
    fn flash(&mut self) {
        self.led.set_high();
        self.led_off_at = Some(Instant::now() + FLASH);
    }

    // ANCHOR: render
    /// Draws the core's current view at the switch's brightness. The core's
    /// count is unbounded, so the ring is an odometer: each lap of ten fills in
    /// a new colour over the last lap's. 10 is ten green, 11 is one cyan over
    /// nine green, 20 is ten cyan. (Ported from `cpb-counter-http`.)
    fn render(&mut self) {
        let value = self.core.view().value;

        let level = if self.bright { BRIGHT } else { DIM };
        let positive = value >= 0;
        let n = value.unsigned_abs() as usize;

        let mut pixels = [Rgb::default(); PIXELS];
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
        self.show(&pixels);
    }
    // ANCHOR_END: render

    /// Sends the pixels down the WS2812 line.
    fn show(&mut self, pixels: &[Rgb; PIXELS]) {
        // GRB, most significant bit first.
        for (pixel, chunk) in pixels.iter().zip(self.words.chunks_mut(24)) {
            let grb = (u32::from(pixel.g) << 16) | (u32::from(pixel.r) << 8) | u32::from(pixel.b);
            for (bit, word) in chunk.iter_mut().enumerate() {
                *word = if grb & (1 << (23 - bit)) == 0 {
                    T0H
                } else {
                    T1H
                };
            }
        }
        self.words[WORDS - 1] = RES;

        let mut seq_config = SequenceConfig::default();
        seq_config.end_delay = 799; // ~50 us reset
        let sequencer = SingleSequencer::new(&mut self.pwm, &self.words, seq_config);
        let _ = sequencer.start(SingleSequenceMode::Times(1));
        // The sequence takes ~300 us; dropping the sequencer stops it, so wait.
        cortex_m::asm::delay(64_000); // ~1 ms at 64 MHz
        drop(sequencer);
    }
}

/// The colour of lap `lap` (0 for 1..=10, 1 for 11..=20, ...): green, cyan,
/// blue for positive counts, and red, orange, magenta for negative ones, round
/// and round.
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

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let _ = spawner; // no other tasks: the shell is the main task
    {
        use core::mem::MaybeUninit;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        #[allow(static_mut_refs)]
        unsafe {
            HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE);
        }
    }

    // The 32 MHz crystal, so the WS2812 bit timing derived from PWM is accurate.
    let mut config = embassy_nrf::config::Config::default();
    config.hfclk_source = embassy_nrf::config::HfclkSource::ExternalXtal;
    let p = embassy_nrf::init(config);

    // Power the NeoPixels.
    let _power = Output::new(p.P0_06, Level::Low, OutputDrive::Standard);
    let led = Output::new(p.P1_14, Level::Low, OutputDrive::Standard);
    let mut button_a = Debounced::new(Input::new(p.P1_02, Pull::Down));
    let mut button_b = Debounced::new(Input::new(p.P1_15, Pull::Down));
    let mut switch = Debounced::new(Input::new(p.P1_06, Pull::Up));

    let mut pwm_config = PwmConfig::default();
    pwm_config.sequence_load = SequenceLoad::Common;
    pwm_config.prescaler = Prescaler::Div1;
    pwm_config.max_duty = 20;
    let Ok(pwm) = SequencePwm::new_1ch(p.PWM0, p.P0_13, pwm_config) else {
        panic!("PWM init");
    };

    // ANCHOR: main_loop
    let mut shell = Shell {
        core: Core::new(),
        pwm,
        words: [RES; WORDS],
        // The switch reads low in its left position: bright.
        bright: !switch.high,
        led,
        led_off_at: None,
    };

    // Draw the initial view. The core hasn't asked for a render yet, and no
    // event is needed to read its view.
    shell.render();

    loop {
        // Wake when the LED's flash is due to end, if one is showing.
        let led_off_at = shell.led_off_at;
        let flash_ends = async {
            match led_off_at {
                Some(at) => Timer::at(at).await,
                None => core::future::pending().await,
            }
        };

        match select4(
            button_a.changed(),
            button_b.changed(),
            switch.changed(),
            flash_ends,
        )
        .await
        {
            // Buttons are active high: count presses, ignore releases.
            // (The core's `Reset` event has no button here.)
            Either4::First(pressed) => {
                if pressed {
                    shell.flash();
                    shell.process_event(Event::Increment);
                }
            }
            Either4::Second(pressed) => {
                if pressed {
                    shell.flash();
                    shell.process_event(Event::Decrement);
                }
            }
            // Brightness is the shell's own business: redraw the same view.
            Either4::Third(high) => {
                shell.bright = !high;
                shell.render();
            }
            Either4::Fourth(()) => {
                shell.led.set_low();
                shell.led_off_at = None;
            }
        }
    }
    // ANCHOR_END: main_loop
}
