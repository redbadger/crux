//! The Crux core: an ordinary Crux app, written in a `no_std` crate.
//!
//! Nothing in here knows about the board. Button A increments, button B
//! decrements, the slide switch picks the brightness, and every press flashes
//! the red LED for a moment by asking the shell for a `Delay`.

use crux_core::{
    App, Command,
    macros::{Operation, effect},
    render::{RenderOperation, render},
};
use facet::Facet;

/// Number of NeoPixels on the board.
pub const PIXELS: usize = 10;

/// How long the red LED stays on after a press.
const FLASH_MILLIS: u32 = 120;

#[derive(Debug)]
pub enum Event {
    ButtonA,
    ButtonB,
    /// The slide switch moved; `true` is the left position.
    Switch(bool),
    /// Internal: the flash delay has elapsed.
    FlashDone(u32),
}

/// A custom async effect: wait this long, then answer with `()`.
/// The firmware shell resolves it with `embassy_time`.
#[derive(Operation, Debug)]
#[operation(request, output = ())]
pub struct Delay {
    pub millis: u32,
}

// Plain `#[effect]`: `#[effect(facet_typegen)]` would also emit the FFI enum
// and `EffectFFI` impl, which need crux_core's `bridge` feature (std).
#[effect]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    Delay(Delay),
}

#[derive(Default)]
pub struct Model {
    count: i32,
    bright: bool,
    /// Bumped on every press, so only the latest flash turns the LED off.
    flash: u32,
    led_on: bool,
}

/// One RGB colour per pixel.
///
/// Derives `Facet` purely as an experiment: does facet's derive work in a
/// `no_std` user crate? (It is not needed for anything here.)
#[derive(Facet, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Facet, Clone, Debug, Default, PartialEq, Eq)]
pub struct ViewModel {
    pub pixels: [Rgb; PIXELS],
    pub led_on: bool,
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
            Event::ButtonA => {
                model.count = (model.count + 1).min(PIXELS as i32);
                flash(model)
            }
            Event::ButtonB => {
                model.count = (model.count - 1).max(-(PIXELS as i32));
                flash(model)
            }
            Event::Switch(left) => {
                model.bright = left;
                render()
            }
            Event::FlashDone(id) => {
                if id == model.flash {
                    model.led_on = false;
                }
                render()
            }
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        let level = if model.bright { 40 } else { 6 };
        let colour = if model.count >= 0 {
            Rgb { r: 0, g: level, b: 0 }
        } else {
            Rgb { r: level, g: 0, b: 0 }
        };
        let lit = model.count.unsigned_abs() as usize;

        let mut pixels = [Rgb::default(); PIXELS];
        for pixel in pixels.iter_mut().take(lit) {
            *pixel = colour;
        }

        ViewModel {
            pixels,
            led_on: model.led_on,
        }
    }
}

/// Turn the LED on now, render, and in parallel run an async task which
/// waits for the shell to resolve a `Delay` and then sends `FlashDone`.
fn flash(model: &mut Model) -> Command<Effect, Event> {
    model.flash = model.flash.wrapping_add(1);
    model.led_on = true;
    let id = model.flash;

    let blink = Command::new(move |ctx| async move {
        ctx.request_from_shell(Delay {
            millis: FLASH_MILLIS,
        })
        .await;
        ctx.send_event(Event::FlashDone(id));
    });

    render().and(blink)
}
