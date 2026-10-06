//! spike(no_std): phase 2 check of the whole link, without Crux.
//!
//! When the gateway connects and subscribes, the device sends a raw
//! `ToGateway::Http` GET for `https://crux-counter.fly.dev/` (the request crux_http's
//! no_std builder would make), and shows what comes back. Button A sends it again;
//! button B opens the SSE stream.
//!
//! Pixel 0: dim blue advertising, cyan connected and subscribed, red if the radio failed.
//! Pixel 1: amber while a GET is in flight, then green for a 2xx answer, red for anything
//! else. Pixel 2 flashes violet for each SSE chunk.

#![no_std]
#![no_main]

extern crate alloc;

#[path = "../link.rs"]
mod link;
#[path = "../neopixel.rs"]
mod neopixel;

use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_futures::select::{Either3, select3};
use embassy_nrf::gpio::{Input, Level, Output, OutputDrive, Pull};
use embassy_time::Timer;
use embedded_alloc::LlffHeap as Heap;
use panic_halt as _;

use cpb_protocol::{SseRequest, ToDevice, ToGateway};
use crux_http::protocol::{HttpRequest, HttpResult};
use link::{INCOMING, LinkEvent, OUTGOING};
use neopixel::{Frame, NeoPixels, PIXELS};

#[global_allocator]
static HEAP: Heap = Heap::empty();
const HEAP_SIZE: usize = 32 * 1024;

const OFF: [u8; 3] = [0, 0, 0];

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
    let _power = Output::new(p.P0_06, Level::Low, OutputDrive::Standard);
    let mut pixels = NeoPixels::new(p.PWM0, p.P0_13);
    let mut frame: Frame = [OFF; PIXELS];
    frame[0] = [6, 6, 6];
    pixels.show(&frame);

    let mut button_a = Input::new(p.P1_02, Pull::Down);
    let mut button_b = Input::new(p.P1_15, Pull::Down);

    let Some(controller) = link::controller(spawner, radio!(p)) else {
        pixels.show(&[[24, 0, 0]; PIXELS]);
        return;
    };

    join(link::run(controller), async {
        let mut next_id: u16 = 0;
        let mut up = false;
        loop {
            let event = select3(
                INCOMING.receive(),
                button_a.wait_for_rising_edge(),
                button_b.wait_for_rising_edge(),
            )
            .await;
            match event {
                Either3::First(LinkEvent::Advertising) => {
                    up = false;
                    frame = [OFF; PIXELS];
                    frame[0] = [0, 0, 8];
                }
                Either3::First(LinkEvent::Up) => {
                    up = true;
                    frame[0] = [0, 10, 10];
                    next_id = next_id.wrapping_add(1);
                    get(next_id, &mut frame).await;
                }
                Either3::First(LinkEvent::Down) => {
                    up = false;
                    frame = [OFF; PIXELS];
                }
                Either3::First(LinkEvent::Failed) => frame = [[24, 0, 0]; PIXELS],
                Either3::First(LinkEvent::Message(ToDevice::Http { result, .. })) => {
                    frame[1] = match result {
                        HttpResult::Ok(response) if (200..300).contains(&response.status) => {
                            [0, 16, 0]
                        }
                        _ => [16, 0, 0],
                    };
                }
                Either3::First(LinkEvent::Message(ToDevice::SseChunk { .. })) => {
                    frame[2] = [10, 0, 16];
                    pixels.show(&frame);
                    Timer::after_millis(100).await;
                    frame[2] = OFF;
                }
                Either3::First(LinkEvent::Message(ToDevice::SseDone { .. })) => {}
                Either3::Second(()) if up => {
                    next_id = next_id.wrapping_add(1);
                    get(next_id, &mut frame).await;
                }
                Either3::Third(()) if up => {
                    next_id = next_id.wrapping_add(1);
                    OUTGOING
                        .send(ToGateway::Sse {
                            id: next_id,
                            request: SseRequest {
                                url: "https://crux-counter.fly.dev/sse".into(),
                            },
                        })
                        .await;
                }
                Either3::Second(()) | Either3::Third(()) => {}
            }
            pixels.show(&frame);
            // Crude debounce for the buttons.
            Timer::after_millis(30).await;
        }
    })
    .await;
}

async fn get(id: u16, frame: &mut Frame) {
    frame[1] = [16, 8, 0];
    OUTGOING
        .send(ToGateway::Http {
            id,
            request: HttpRequest::get("https://crux-counter.fly.dev/").build(),
        })
        .await;
}
