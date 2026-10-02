//! spike(no_std): the firmware "shell" for a Crux core on the Circuit
//! Playground Bluefruit. It calls `Core` directly; there is no FFI bridge.
//!
//! Pins (CircuitPython `ports/nordic/boards/circuitplayground_bluefruit`):
//!   button A    P1.02  (active high, needs pull-down)
//!   button B    P1.15  (active high, needs pull-down)
//!   slide sw.   P1.06  (needs pull-up)
//!   red LED     P1.14  (D13, active high)
//!   NeoPixels   P0.13  (10 x WS2812-style)
//!   power ctl   P0.06  (low = NeoPixels and sensors powered)

#![no_std]
#![no_main]

extern crate alloc;

mod app;

use alloc::vec::Vec;

use crux_core::Core;
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

use app::{Counter, Effect, Event, PIXELS, ViewModel};

#[global_allocator]
static HEAP: Heap = Heap::empty();

/// Heap for Crux: boxed futures, effect vectors, channel queues.
/// The host heap probe (heap-probe/, 32-bit wasm) measures ~0.6 KB idle and
/// ~1.5 KB per press whose flash is still in flight, so 32 KB covers ~20
/// overlapping presses plus allocator overhead. RAM is not scarce here.
const HEAP_SIZE: usize = 32 * 1024;

// WS2812 timing with PWM at 16 MHz, 20 ticks per bit (1.25 us). The high bit
// inverts polarity, so the line starts high.
const T1H: u16 = 0x8000 | 13;
const T0H: u16 = 0x8000 | 7;
const RES: u16 = 0x8000;
const WORDS: usize = PIXELS * 24 + 1;

/// How long an input must settle after an edge before its level is believed.
const DEBOUNCE: Duration = Duration::from_millis(30);

type Pwm<'a> = SequencePwm<'a>;

struct Shell<'a> {
    core: Core<Counter>,
    led: Output<'a>,
    pwm: Pwm<'a>,
    words: [u16; WORDS],
    /// Outstanding `Delay` requests, with the instant each is due.
    delays: Vec<(Instant, crux_core::Request<app::Delay>)>,
}

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
    fn handle(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::Render(_) => {
                    let view = self.core.view();
                    self.render(&view);
                }
                Effect::Delay(request) => {
                    let due = Instant::now() + Duration::from_millis(request.operation.millis.into());
                    self.delays.push((due, request));
                }
            }
        }
    }

    fn resolve_due(&mut self) {
        let now = Instant::now();
        let mut i = 0;
        while i < self.delays.len() {
            if self.delays[i].0 <= now {
                let (_, mut request) = self.delays.swap_remove(i);
                let effects = self
                    .core
                    .resolve(&mut request, ())
                    .unwrap_or_default();
                self.handle(effects);
            } else {
                i += 1;
            }
        }
    }

    fn next_due(&self) -> Option<Instant> {
        self.delays.iter().map(|(due, _)| *due).min()
    }

    fn render(&mut self, view: &ViewModel) {
        self.led.set_level(if view.led_on { Level::High } else { Level::Low });

        // GRB, most significant bit first.
        for (pixel, chunk) in view.pixels.iter().zip(self.words.chunks_mut(24)) {
            let grb = (u32::from(pixel.g) << 16) | (u32::from(pixel.r) << 8) | u32::from(pixel.b);
            for (bit, word) in chunk.iter_mut().enumerate() {
                *word = if grb & (1 << (23 - bit)) == 0 { T0H } else { T1H };
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

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
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

    let mut shell = Shell {
        core: Core::new(),
        led,
        pwm,
        words: [RES; WORDS],
        delays: Vec::new(),
    };

    let effects = shell.core.process_event(Event::Switch(!switch.high));
    shell.handle(effects);

    loop {
        let due = shell.next_due();
        let timer = async {
            match due {
                Some(at) => Timer::at(at).await,
                None => core::future::pending().await,
            }
        };

        let event = match select4(
            button_a.changed(),
            button_b.changed(),
            switch.changed(),
            timer,
        )
        .await
        {
            // Buttons are active high: count presses, ignore releases.
            Either4::First(pressed) => pressed.then_some(Event::ButtonA),
            Either4::Second(pressed) => pressed.then_some(Event::ButtonB),
            Either4::Third(high) => Some(Event::Switch(!high)),
            Either4::Fourth(()) => None,
        };

        if let Some(event) = event {
            let effects = shell.core.process_event(event);
            shell.handle(effects);
        }
        shell.resolve_due();
    }
}
