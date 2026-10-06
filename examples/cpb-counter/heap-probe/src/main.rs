//! Host-side heap probe for the cpb-counter core. Uses the firmware's `app.rs`
//! unchanged, drives it the way the shell does, and reports peak heap use.
//! Host pointers are 8 bytes against 4 on the nRF52840, so treat the result as
//! an upper bound.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

#[path = "../../src/app.rs"]
mod app;

use app::{Counter, Effect, Event};
use crux_core::Core;

struct Tracking;
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Tracking {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let now = CURRENT.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
        PEAK.fetch_max(now, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Tracking = Tracking;

fn main() {
    let base = CURRENT.load(Ordering::Relaxed);
    PEAK.store(base, Ordering::Relaxed);

    let core: Core<Counter> = Core::new();
    let mut delays = Vec::new();
    let mut renders = 0;

    let mut handle = |effects: Vec<Effect>, delays: &mut Vec<_>| {
        for effect in effects {
            match effect {
                Effect::Render(_) => {
                    let _ = core.view();
                    renders += 1;
                }
                Effect::Delay(request) => delays.push(request),
            }
        }
    };

    handle(core.process_event(Event::Switch(true)), &mut delays);

    // Worst case for the shell: a burst of presses before any delay resolves.
    for burst in [1usize, 5, 20, 20, 20, 1, 1] {
        for i in 0..burst {
            let event = if i % 2 == 0 {
                Event::ButtonA
            } else {
                Event::ButtonB
            };
            handle(core.process_event(event), &mut delays);
        }
        let in_flight = delays.len();
        let peak_in_flight = PEAK.load(Ordering::Relaxed) - base;
        for mut request in std::mem::take(&mut delays) {
            let effects = core.resolve(&mut request, ()).expect("resolves");
            handle(effects, &mut delays);
        }
        println!(
            "burst of {burst:>2} presses: {in_flight:>2} delays in flight, peak heap so far {peak_in_flight} B, live after settle {} B",
            CURRENT.load(Ordering::Relaxed) - base
        );
    }
    println!("renders: {renders}, view: {:?}", core.view());
}
