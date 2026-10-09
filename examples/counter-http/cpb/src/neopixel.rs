//! The ten WS2812-style `NeoPixels` on P0.13, driven by `SequencePwm` (PWM0 at 16 MHz,
//! 20 ticks per bit), as in cpb-counter. P0.06 must be held low or they have no power.
//!
//! Unlike cpb-counter, HFCLK is not forced to the crystal here: MPSL owns the CLOCK
//! peripheral and requests the crystal when the radio needs it. WS2812 bit timing
//! tolerates ±150 ns on a 1.25 µs bit, far more than the internal oscillator's error.

use embassy_nrf::Peri;
use embassy_nrf::gpio::Pin;
use embassy_nrf::pwm::{
    Config as PwmConfig, Instance, Prescaler, SequenceConfig, SequenceLoad, SequencePwm,
    SingleSequenceMode, SingleSequencer,
};

pub const PIXELS: usize = 10;

// The high bit inverts polarity, so the line starts high.
// WS2812 timing in PWM ticks; the duty reads best in decimal next to the polarity bit.
#[allow(clippy::decimal_bitwise_operands)]
const T1H: u16 = 0x8000 | 13;
// WS2812 timing in PWM ticks; the duty reads best in decimal next to the polarity bit.
#[allow(clippy::decimal_bitwise_operands)]
const T0H: u16 = 0x8000 | 7;
const RES: u16 = 0x8000;
const WORDS: usize = PIXELS * 24 + 1;

/// One colour per pixel, `[r, g, b]`.
pub type Frame = [[u8; 3]; PIXELS];

pub struct NeoPixels<'d> {
    pwm: SequencePwm<'d>,
    /// In RAM, as `EasyDMA` requires.
    words: [u16; WORDS],
}

impl<'d> NeoPixels<'d> {
    pub fn new(pwm: Peri<'d, impl Instance>, pin: Peri<'d, impl Pin>) -> Self {
        let mut config = PwmConfig::default();
        config.sequence_load = SequenceLoad::Common;
        config.prescaler = Prescaler::Div1;
        config.max_duty = 20;
        let Ok(pwm) = SequencePwm::new_1ch(pwm, pin, config) else {
            panic!("PWM init");
        };
        Self {
            pwm,
            words: [RES; WORDS],
        }
    }

    pub fn show(&mut self, frame: &Frame) {
        // GRB, most significant bit first.
        for (&[r, g, b], chunk) in frame.iter().zip(self.words.chunks_mut(24)) {
            let grb = (u32::from(g) << 16) | (u32::from(r) << 8) | u32::from(b);
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
